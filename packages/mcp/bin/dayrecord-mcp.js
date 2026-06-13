#!/usr/bin/env node
/**
 * @dayrecord/mcp — npx-friendly MCP launcher.
 * Uses a local dayrecord binary, or downloads the latest GitHub Release on first run.
 */
const { spawn, spawnSync } = require('child_process');
const crypto = require('crypto');
const fs = require('fs');
const http = require('http');
const https = require('https');
const os = require('os');
const path = require('path');

const REPO = process.env.DAYRECORD_GITHUB_REPO || 'mikaku9944/dayrecord';
const VERSION_PIN = process.env.DAYRECORD_VERSION || '';

function exists(p) {
  try {
    return fs.existsSync(p);
  } catch {
    return false;
  }
}

function installDir() {
  if (process.platform === 'win32') {
    const base = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local');
    return path.join(base, 'Programs', 'dayrecord', 'bin');
  }
  return path.join(os.homedir(), '.local', 'share', 'dayrecord', 'bin');
}

function binName() {
  return process.platform === 'win32' ? 'dayrecord.exe' : 'dayrecord';
}

function defaultBinPath() {
  return path.join(installDir(), binName());
}

function candidatePaths() {
  const candidates = [];
  if (process.env.DAYRECORD_BIN) candidates.push(process.env.DAYRECORD_BIN);
  candidates.push(defaultBinPath());
  if (process.platform === 'win32') {
    const legacy = path.join(
      process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local'),
      'Programs',
      'dayrecord',
      'dayrecord.exe'
    );
    candidates.push(legacy);
  } else {
    candidates.push(path.join(os.homedir(), '.local', 'bin', 'dayrecord'));
  }
  return candidates;
}

function findExistingBinary() {
  for (const p of candidatePaths()) {
    if (p && exists(p)) return p;
  }
  return null;
}

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    const lib = url.startsWith('https') ? https : http;
    const req = lib.get(url, { headers: { 'User-Agent': 'dayrecord-mcp' } }, (res) => {
      if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        fetchJson(res.headers.location).then(resolve, reject);
        return;
      }
      const chunks = [];
      res.on('data', (c) => chunks.push(c));
      res.on('end', () => {
        try {
          resolve(JSON.parse(Buffer.concat(chunks).toString('utf8')));
        } catch (e) {
          reject(e);
        }
      });
    });
    req.on('error', reject);
  });
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const lib = url.startsWith('https') ? https : http;
    const file = fs.createWriteStream(dest);
    const req = lib.get(url, { headers: { 'User-Agent': 'dayrecord-mcp' } }, (res) => {
      if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        file.close();
        fs.unlinkSync(dest);
        downloadFile(res.headers.location, dest).then(resolve, reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        return;
      }
      res.pipe(file);
      file.on('finish', () => file.close(() => resolve(dest)));
    });
    req.on('error', reject);
    file.on('error', reject);
  });
}

function sha256File(filePath) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(filePath));
  return hash.digest('hex');
}

function parseChecksumLine(checksumsText, archiveName) {
  for (const line of checksumsText.split(/\r?\n/)) {
    if (line.includes(archiveName)) {
      return line.split(/\s+/)[0].toLowerCase();
    }
  }
  return null;
}

function platformArtifact(version) {
  const { platform, arch } = process;
  if (platform === 'win32') {
    return {
      archive: `dayrecord-${version}-x86_64-pc-windows-msvc.zip`,
      bin: 'dayrecord.exe',
    };
  }
  if (platform === 'darwin' && arch === 'arm64') {
    return {
      archive: `dayrecord-${version}-aarch64-apple-darwin.tar.gz`,
      bin: 'dayrecord',
    };
  }
  if (platform === 'linux' && (arch === 'x64' || arch === 'amd64')) {
    return {
      archive: `dayrecord-${version}-x86_64-unknown-linux-gnu.tar.gz`,
      bin: 'dayrecord',
    };
  }
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

async function latestVersion() {
  if (VERSION_PIN) return VERSION_PIN.replace(/^v/, '');
  const release = await fetchJson(`https://api.github.com/repos/${REPO}/releases/latest`);
  return String(release.tag_name).replace(/^v/, '');
}

function extractArchive(archivePath, destDir, archiveName) {
  fs.mkdirSync(destDir, { recursive: true });
  if (archiveName.endsWith('.zip')) {
    const ps = `Expand-Archive -Path '${archivePath.replace(/'/g, "''")}' -DestinationPath '${destDir.replace(/'/g, "''")}' -Force`;
    const r = spawnSync('powershell', ['-NoProfile', '-Command', ps], { stdio: 'inherit' });
    if (r.status !== 0) throw new Error('Expand-Archive failed');
    return;
  }
  if (archiveName.endsWith('.tar.gz')) {
    const r = spawnSync('tar', ['xzf', archivePath, '-C', destDir], { stdio: 'inherit' });
    if (r.status !== 0) throw new Error('tar extract failed');
    return;
  }
  throw new Error(`Unknown archive type: ${archiveName}`);
}

function findBinaryInTree(root, name) {
  const stack = [root];
  while (stack.length) {
    const dir = stack.pop();
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) stack.push(full);
      else if (entry.isFile() && entry.name === name) return full;
    }
  }
  return null;
}

async function ensureBinary() {
  const existing = findExistingBinary();
  if (existing) return existing;

  const version = await latestVersion();
  const { archive, bin } = platformArtifact(version);
  const baseUrl = `https://github.com/${REPO}/releases/download/v${version}`;
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'dayrecord-mcp-'));
  const archivePath = path.join(tmp, archive);
  const checksumsPath = path.join(tmp, 'SHA256SUMS.txt');

  process.stderr.write(`@dayrecord/mcp: downloading ${archive}...\n`);
  await downloadFile(`${baseUrl}/${archive}`, archivePath);
  await downloadFile(`${baseUrl}/SHA256SUMS.txt`, checksumsPath);

  const expected = parseChecksumLine(fs.readFileSync(checksumsPath, 'utf8'), archive);
  if (!expected) throw new Error(`SHA256SUMS.txt missing entry for ${archive}`);
  const actual = sha256File(archivePath);
  if (expected !== actual) throw new Error('checksum mismatch');

  const extractDir = path.join(tmp, 'extract');
  extractArchive(archivePath, extractDir, archive);
  const found = findBinaryInTree(extractDir, bin);
  if (!found) throw new Error(`could not find ${bin} in archive`);

  const dest = defaultBinPath();
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(found, dest);
  if (process.platform !== 'win32') fs.chmodSync(dest, 0o755);

  process.stderr.write(`@dayrecord/mcp: installed ${dest}\n`);
  return dest;
}

function runMcp(bin) {
  const isMcpAlias = /dayrecord-mcp(\.exe)?$/i.test(path.basename(bin));
  const args = isMcpAlias ? [] : ['mcp'];
  const child = spawn(bin, args, { stdio: 'inherit', windowsHide: true });
  child.on('error', (err) => {
    console.error(`@dayrecord/mcp: failed to start ${bin}: ${err.message}`);
    process.exit(1);
  });
  child.on('exit', (code, signal) => {
    if (signal) process.kill(process.pid, signal);
    process.exit(code ?? 1);
  });
}

ensureBinary()
  .then(runMcp)
  .catch((err) => {
    console.error(`@dayrecord/mcp: ${err.message}`);
    process.exit(1);
  });
