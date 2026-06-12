//! macOS keyboard capture via CGEventTap (CoreGraphics).
//!
//! Mirrors the Windows `keyboard_hook.rs` design:
//! - Stores `Sender<KeyEvent>` in a global static (idempotent start).
//! - Spawns a dedicated thread that creates a listen-only CGEventTap.
//! - Maps macOS virtual key codes (Carbon HIToolbox) to `KeyEventKind`.
//! - Only captures: a-z (lowercase), 0-9, Space, Enter, Backspace, Tab, Cmd+V (Paste).
//! - Requires "Input Monitoring" privacy permission in System Settings.

use chrono::Utc;
use core_foundation::runloop::CFRunLoop;
use core_foundation::string::CFStringRef;
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use dayrecord_core::models::{KeyEvent, KeyEventKind};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

// kCFRunLoopCommonModes is not exported by core-foundation 0.9; declare it here.
extern "C" {
    static kCFRunLoopCommonModes: CFStringRef;
}

static TX: Mutex<Option<Sender<KeyEvent>>> = Mutex::new(None);

/// Install a CGEventTap keyboard hook and begin sending `KeyEvent` values
/// through `tx`. Idempotent — calling `start` a second time is a no-op.
pub fn start(tx: Sender<KeyEvent>) -> Result<(), String> {
    let mut guard = TX.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Ok(());
    }
    *guard = Some(tx);
    drop(guard);

    std::thread::spawn(|| {
        if let Err(e) = run_event_tap() {
            tracing::error!("CGEventTap failed: {e}");
        }
    });
    Ok(())
}

fn run_event_tap() -> Result<(), String> {
    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::KeyDown],
        |_proxy, _etype, event| {
            let keycode =
                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
            let flags = event.get_flags();
            let cmd = flags.contains(CGEventFlags::CGEventFlagCommand);

            if let Some(kind) = keycode_to_kind(keycode, cmd) {
                if let Ok(guard) = TX.lock() {
                    if let Some(tx) = guard.as_ref() {
                        let _ = tx.send(KeyEvent {
                            at: Utc::now(),
                            kind,
                        });
                    }
                }
            }
            None // ListenOnly — never modify the event
        },
    )
    .map_err(|_| {
        let msg = "CGEventTap::new failed — Input Monitoring permission required.\n\
                   To grant permission:\n\
                   \x20 1. Open System Settings → Privacy & Security → Input Monitoring\n\
                   \x20 2. Click '+' and add your terminal app:\n\
                   \x20    • Terminal.app: /System/Applications/Utilities/Terminal.app\n\
                   \x20    • iTerm2:       /Applications/iTerm.app\n\
                   \x20    • VS Code:      the Visual Studio Code app\n\
                   \x20 3. Restart the terminal and run `dayrecord daemon` again.";
        tracing::error!("{msg}");
        msg.to_string()
    })?;

    tap.enable();

    let source = tap
        .mach_port
        .create_runloop_source(0)
        .map_err(|_| "create_runloop_source failed".to_string())?;
    let current = CFRunLoop::get_current();
    unsafe {
        current.add_source(&source, kCFRunLoopCommonModes);
    }
    // Note: kCFRunLoopCommonModes is accessed unsafely as an extern static.

    tracing::info!("macOS CGEventTap keyboard hook started");
    CFRunLoop::run_current();

    // CFRunLoop::run_current() only returns if the run loop is stopped,
    // which normally doesn't happen for a long-lived daemon.
    Ok(())
}

// ── Carbon HIToolbox virtual key codes ──────────────────────────────────────
//
// These are NOT sequential. Values from:
//   Carbon.framework/Frameworks/HIToolbox.framework/Headers/Events.h
//
// Letters:  A=0  B=11 C=8  D=2  E=14 F=3  G=5  H=4  I=34 J=38
//           K=40 L=37 M=46 N=45 O=31 P=35 Q=12 R=15 S=1  T=17
//           U=32 V=9  W=13 X=7  Y=16 Z=6
//
// Digits:   0=29 1=18 2=19 3=20 4=21 5=23 6=22 7=26 8=28 9=25
//
// Special:  Return=36 Tab=48 Space=49 Delete(Backspace)=51

const VK_RETURN: u16 = 36;
const VK_TAB: u16 = 48;
const VK_SPACE: u16 = 49;
const VK_DELETE: u16 = 51; // macOS "Delete" = PC "Backspace"

const VK_ANSI_V: u16 = 9;

/// Map a macOS virtual key code + modifier state to a semantic `KeyEventKind`.
///
/// Mirrors the Windows `vk_to_kind()`: only alphanumeric + 4 special keys + Paste.
/// All letters are lowercased. Shift/Ctrl/Alt are invisible.
fn keycode_to_kind(vk: u16, cmd: bool) -> Option<KeyEventKind> {
    // Cmd+V → Paste (macOS equivalent of Ctrl+V)
    if vk == VK_ANSI_V && cmd {
        return Some(KeyEventKind::Paste);
    }

    // Special keys
    match vk {
        x if x == VK_SPACE => return Some(KeyEventKind::Space),
        x if x == VK_RETURN => return Some(KeyEventKind::Enter),
        x if x == VK_DELETE => return Some(KeyEventKind::Backspace),
        x if x == VK_TAB => return Some(KeyEventKind::Tab),
        _ => {}
    }

    // Letters (a–z, lowercase)
    let ch = match vk {
        0 => Some('a'),
        11 => Some('b'),
        8 => Some('c'),
        2 => Some('d'),
        14 => Some('e'),
        3 => Some('f'),
        5 => Some('g'),
        4 => Some('h'),
        34 => Some('i'),
        38 => Some('j'),
        40 => Some('k'),
        37 => Some('l'),
        46 => Some('m'),
        45 => Some('n'),
        31 => Some('o'),
        35 => Some('p'),
        12 => Some('q'),
        15 => Some('r'),
        1 => Some('s'),
        17 => Some('t'),
        32 => Some('u'),
        9 => Some('v'),
        13 => Some('w'),
        7 => Some('x'),
        16 => Some('y'),
        6 => Some('z'),
        _ => None,
    };
    if let Some(c) = ch {
        return Some(KeyEventKind::Char(c));
    }

    // Digits (0–9)
    let digit = match vk {
        29 => Some('0'),
        18 => Some('1'),
        19 => Some('2'),
        20 => Some('3'),
        21 => Some('4'),
        23 => Some('5'),
        22 => Some('6'),
        26 => Some('7'),
        28 => Some('8'),
        25 => Some('9'),
        _ => None,
    };
    if let Some(d) = digit {
        return Some(KeyEventKind::Char(d));
    }

    // All other keys silently ignored (function keys, arrows, punctuation, etc.)
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_mapping() {
        assert_eq!(keycode_to_kind(0, false), Some(KeyEventKind::Char('a')));
        assert_eq!(keycode_to_kind(1, false), Some(KeyEventKind::Char('s')));
        assert_eq!(keycode_to_kind(6, false), Some(KeyEventKind::Char('z')));
        assert_eq!(keycode_to_kind(12, false), Some(KeyEventKind::Char('q')));
    }

    #[test]
    fn digit_mapping() {
        assert_eq!(keycode_to_kind(29, false), Some(KeyEventKind::Char('0')));
        assert_eq!(keycode_to_kind(18, false), Some(KeyEventKind::Char('1')));
        assert_eq!(keycode_to_kind(25, false), Some(KeyEventKind::Char('9')));
    }

    #[test]
    fn special_keys() {
        assert_eq!(keycode_to_kind(VK_SPACE, false), Some(KeyEventKind::Space));
        assert_eq!(
            keycode_to_kind(VK_RETURN, false),
            Some(KeyEventKind::Enter)
        );
        assert_eq!(
            keycode_to_kind(VK_DELETE, false),
            Some(KeyEventKind::Backspace)
        );
        assert_eq!(keycode_to_kind(VK_TAB, false), Some(KeyEventKind::Tab));
    }

    #[test]
    fn cmd_v_is_paste() {
        assert_eq!(
            keycode_to_kind(VK_ANSI_V, true),
            Some(KeyEventKind::Paste)
        );
        // V without Cmd is just the letter 'v'
        assert_eq!(
            keycode_to_kind(VK_ANSI_V, false),
            Some(KeyEventKind::Char('v'))
        );
    }

    #[test]
    fn unknown_keys_return_none() {
        // F1 = 0x7A = 122
        assert_eq!(keycode_to_kind(122, false), None);
        // Left arrow = 0x7B = 123
        assert_eq!(keycode_to_kind(123, false), None);
        // Semicolon = 0x29 = 41
        assert_eq!(keycode_to_kind(41, false), None);
    }
}
