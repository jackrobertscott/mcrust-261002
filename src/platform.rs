//! macOS platform layer: window, OpenGL context and input, implemented directly
//! on top of the Objective-C runtime (libobjc) and system frameworks. No crates.
#![allow(non_upper_case_globals, dead_code)]

use std::ffi::{c_char, c_void, CString};

pub type Id = *mut c_void;
type Sel = *mut c_void;
const NIL: Id = std::ptr::null_mut();

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NSPoint {
    pub x: f64,
    pub y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NSSize {
    pub w: f64,
    pub h: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NSRect {
    pub origin: NSPoint,
    pub size: NSSize,
}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
}

#[link(name = "Cocoa", kind = "framework")]
unsafe extern "C" {
    static NSDefaultRunLoopMode: Id;
}
#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGAssociateMouseAndMouseCursorPosition(connected: i32) -> i32;
    fn CGWarpMouseCursorPosition(p: NSPoint) -> i32;
}

fn sel(name: &str) -> Sel {
    let c = CString::new(name).unwrap();
    unsafe { sel_registerName(c.as_ptr()) }
}
fn class(name: &str) -> Id {
    let c = CString::new(name).unwrap();
    unsafe { objc_getClass(c.as_ptr()) }
}

/// Send an Objective-C message. Usage: `msg!(ret_ty; obj, "sel:with:", a: T, b: U)`.
macro_rules! msg {
    ($ret:ty; $obj:expr, $sel:expr $(, $arg:expr => $t:ty)*) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel $(, $t)*) -> $ret =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            f($obj, sel($sel) $(, $arg)*)
        }
    }};
}

fn msg_rect(obj: Id, s: &str) -> NSRect {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        let f: unsafe extern "C" fn(Id, Sel) -> NSRect =
            std::mem::transmute(objc_msgSend_stret as unsafe extern "C" fn());
        #[cfg(not(target_arch = "x86_64"))]
        let f: unsafe extern "C" fn(Id, Sel) -> NSRect =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(obj, sel(s))
    }
}
fn msg_rect_arg(obj: Id, s: &str, r: NSRect) -> NSRect {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        let f: unsafe extern "C" fn(Id, Sel, NSRect) -> NSRect =
            std::mem::transmute(objc_msgSend_stret as unsafe extern "C" fn());
        #[cfg(not(target_arch = "x86_64"))]
        let f: unsafe extern "C" fn(Id, Sel, NSRect) -> NSRect =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(obj, sel(s), r)
    }
}

fn nsstring(s: &str) -> Id {
    let c = CString::new(s).unwrap();
    msg!(Id; class("NSString"), "stringWithUTF8String:", c.as_ptr() => *const c_char)
}

// ---- Key codes (macOS virtual key codes) ----
pub mod key {
    pub const A: u16 = 0;
    pub const S: u16 = 1;
    pub const D: u16 = 2;
    pub const F: u16 = 3;
    pub const H: u16 = 4;
    pub const G: u16 = 5;
    pub const Z: u16 = 6;
    pub const X: u16 = 7;
    pub const C: u16 = 8;
    pub const V: u16 = 9;
    pub const B: u16 = 11;
    pub const Q: u16 = 12;
    pub const W: u16 = 13;
    pub const E: u16 = 14;
    pub const R: u16 = 15;
    pub const Y: u16 = 16;
    pub const T: u16 = 17;
    pub const N1: u16 = 18;
    pub const N2: u16 = 19;
    pub const N3: u16 = 20;
    pub const N4: u16 = 21;
    pub const N6: u16 = 22;
    pub const N5: u16 = 23;
    pub const N9: u16 = 25;
    pub const N7: u16 = 26;
    pub const N8: u16 = 28;
    pub const N0: u16 = 29;
    pub const O: u16 = 31;
    pub const U: u16 = 32;
    pub const I: u16 = 34;
    pub const P: u16 = 35;
    pub const RETURN: u16 = 36;
    pub const L: u16 = 37;
    pub const J: u16 = 38;
    pub const K: u16 = 40;
    pub const N: u16 = 45;
    pub const M: u16 = 46;
    pub const TAB: u16 = 48;
    pub const SPACE: u16 = 49;
    pub const BACKSPACE: u16 = 51;
    pub const ESCAPE: u16 = 53;
    pub const F5: u16 = 96;
    pub const F3: u16 = 99;
    pub const F2: u16 = 120;
    pub const F1: u16 = 122;
    pub const F11: u16 = 103;
    pub const LEFT: u16 = 123;
    pub const RIGHT: u16 = 124;
    pub const DOWN: u16 = 125;
    pub const UP: u16 = 126;
    // Synthetic codes for modifier keys (tracked from flagsChanged)
    pub const SHIFT: u16 = 200;
    pub const CONTROL: u16 = 201;
    pub const OPTION: u16 = 202;
    pub const COMMAND: u16 = 203;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    KeyDown(u16, bool), // (keycode, is_repeat)
    KeyUp(u16),
    Char(char),
    MouseDown(u8), // 0 left, 1 right, 2 middle
    MouseUp(u8),
    Scroll(f32),
}

pub struct Window {
    app: Id,
    window: Id,
    view: Id,
    ctx: Id,
    pool: Id,
    pub width: u32,  // framebuffer pixels
    pub height: u32, // framebuffer pixels
    pub scale: f32,  // backing scale factor
    pub mouse_x: f32, // framebuffer pixel coords, origin top-left
    pub mouse_y: f32,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub keys: [bool; 256],
    pub buttons: [bool; 3],
    pub events: Vec<Event>,
    pub should_close: bool,
    pub focused: bool,
    mouse_locked: bool,
    cursor_hidden: bool,
    mods: u64,
    last_view_size: (f64, f64),
}

impl Window {
    pub fn new(title: &str, w: f64, h: f64) -> Window {
        let pool = msg!(Id; msg!(Id; class("NSAutoreleasePool"), "alloc"), "init");
        let app = msg!(Id; class("NSApplication"), "sharedApplication");
        msg!(bool; app, "setActivationPolicy:", 0i64 => i64);

        // Menu bar with a Quit item so Cmd+Q works natively.
        let menubar = msg!(Id; msg!(Id; class("NSMenu"), "alloc"), "init");
        let app_item = msg!(Id; msg!(Id; class("NSMenuItem"), "alloc"), "init");
        msg!((); menubar, "addItem:", app_item => Id);
        msg!((); app, "setMainMenu:", menubar => Id);
        let app_menu = msg!(Id; msg!(Id; class("NSMenu"), "alloc"), "init");
        let quit = msg!(Id; msg!(Id; class("NSMenuItem"), "alloc"),
            "initWithTitle:action:keyEquivalent:",
            nsstring(&format!("Quit {title}")) => Id, sel("terminate:") => Sel, nsstring("q") => Id);
        msg!((); app_menu, "addItem:", quit => Id);
        msg!((); app_item, "setSubmenu:", app_menu => Id);

        let rect = NSRect { origin: NSPoint { x: 0.0, y: 0.0 }, size: NSSize { w, h } };
        let style: u64 = 1 | 2 | 4 | 8; // titled, closable, miniaturizable, resizable
        let window = msg!(Id; msg!(Id; class("NSWindow"), "alloc"),
            "initWithContentRect:styleMask:backing:defer:",
            rect => NSRect, style => u64, 2u64 => u64, false => bool);
        msg!((); window, "setReleasedWhenClosed:", false => bool);
        msg!((); window, "setTitle:", nsstring(title) => Id);
        msg!((); window, "center");
        msg!((); window, "setAcceptsMouseMovedEvents:", true => bool);
        let min = NSSize { w: 320.0, h: 240.0 };
        msg!((); window, "setContentMinSize:", min => NSSize);

        let view = msg!(Id; window, "contentView");
        msg!((); view, "setWantsBestResolutionOpenGLSurface:", true => bool);

        // OpenGL legacy (2.1) context: lets us use GLSL 1.20 + client arrays.
        let attrs: [u32; 13] = [
            5,  // NSOpenGLPFADoubleBuffer
            73, // NSOpenGLPFAAccelerated
            8, 24, // ColorSize
            11, 8, // AlphaSize
            12, 24, // DepthSize
            99, 0x1000, // OpenGLProfile: Legacy
            0, 0, 0,
        ];
        let pf = msg!(Id; msg!(Id; class("NSOpenGLPixelFormat"), "alloc"),
            "initWithAttributes:", attrs.as_ptr() => *const u32);
        if pf.is_null() {
            panic!("Could not create OpenGL pixel format");
        }
        let ctx = msg!(Id; msg!(Id; class("NSOpenGLContext"), "alloc"),
            "initWithFormat:shareContext:", pf => Id, NIL => Id);
        if ctx.is_null() {
            panic!("Could not create OpenGL context");
        }
        let one: i32 = 1;
        msg!((); ctx, "setValues:forParameter:", &one as *const i32 => *const i32, 222i64 => i64);
        msg!((); window, "makeKeyAndOrderFront:", NIL => Id);
        msg!((); ctx, "setView:", view => Id);
        msg!((); ctx, "makeCurrentContext");
        msg!((); app, "finishLaunching");
        msg!((); app, "activateIgnoringOtherApps:", true => bool);

        let mut win = Window {
            app,
            window,
            view,
            ctx,
            pool,
            width: 1,
            height: 1,
            scale: 1.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            keys: [false; 256],
            buttons: [false; 3],
            events: Vec::new(),
            should_close: false,
            focused: true,
            mouse_locked: false,
            cursor_hidden: false,
            mods: 0,
            last_view_size: (0.0, 0.0),
        };
        win.update_size();
        win
    }

    fn update_size(&mut self) {
        let bounds = msg_rect(self.view, "bounds");
        let backing = msg_rect_arg(self.view, "convertRectToBacking:", bounds);
        let vs = (bounds.size.w, bounds.size.h);
        if vs != self.last_view_size {
            self.last_view_size = vs;
            msg!((); self.ctx, "update");
        }
        self.width = (backing.size.w.round() as u32).max(1);
        self.height = (backing.size.h.round() as u32).max(1);
        self.scale = if bounds.size.w > 0.0 { (backing.size.w / bounds.size.w) as f32 } else { 1.0 };
    }

    /// Pump all pending OS events. Fills `events`, `keys`, mouse deltas.
    pub fn poll(&mut self) {
        self.events.clear();
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        // Drain the autorelease pool every frame.
        msg!((); self.pool, "drain");
        self.pool = msg!(Id; msg!(Id; class("NSAutoreleasePool"), "alloc"), "init");

        let distant_past = msg!(Id; class("NSDate"), "distantPast");
        let mode = unsafe { NSDefaultRunLoopMode };
        loop {
            let ev = msg!(Id; self.app, "nextEventMatchingMask:untilDate:inMode:dequeue:",
                u64::MAX => u64, distant_past => Id, mode => Id, true => bool);
            if ev.is_null() {
                break;
            }
            let ty = msg!(u64; ev, "type");
            let mut forward = true;
            match ty {
                1 | 3 | 25 => {
                    let b = match ty { 1 => 0, 3 => 1, _ => 2 };
                    // Only treat clicks inside the content view as game clicks.
                    if self.event_in_view(ev) || self.mouse_locked {
                        self.buttons[b as usize] = true;
                        self.events.push(Event::MouseDown(b));
                    }
                }
                2 | 4 | 26 => {
                    let b = match ty { 2 => 0, 4 => 1, _ => 2 };
                    if self.buttons[b as usize] {
                        self.buttons[b as usize] = false;
                        self.events.push(Event::MouseUp(b));
                    }
                }
                5 | 6 | 7 | 27 => {
                    let dx = msg!(f64; ev, "deltaX");
                    let dy = msg!(f64; ev, "deltaY");
                    self.mouse_dx += dx as f32;
                    self.mouse_dy += dy as f32;
                }
                10 => {
                    let code = msg!(u16; ev, "keyCode");
                    let repeat = msg!(bool; ev, "isARepeat");
                    let flags = msg!(u64; ev, "modifierFlags");
                    if flags & (1 << 20) != 0 {
                        // Command shortcuts go to the menu (Cmd+Q).
                        if code == key::Q {
                            self.should_close = true;
                        }
                    } else {
                        forward = false;
                        if (code as usize) < 256 {
                            self.keys[code as usize] = true;
                        }
                        self.events.push(Event::KeyDown(code, repeat));
                        let chars = msg!(Id; ev, "characters");
                        if !chars.is_null() {
                            let p = msg!(*const c_char; chars, "UTF8String");
                            if !p.is_null() {
                                let s = unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned();
                                for c in s.chars() {
                                    if !c.is_control() && (c as u32) < 0xF700 {
                                        self.events.push(Event::Char(c));
                                    }
                                }
                            }
                        }
                    }
                }
                11 => {
                    let code = msg!(u16; ev, "keyCode");
                    forward = false;
                    if (code as usize) < 256 {
                        self.keys[code as usize] = false;
                    }
                    self.events.push(Event::KeyUp(code));
                }
                12 => {
                    let flags = msg!(u64; ev, "modifierFlags");
                    let map = [(1u64 << 17, key::SHIFT), (1 << 18, key::CONTROL), (1 << 19, key::OPTION), (1 << 20, key::COMMAND)];
                    for (bit, k) in map {
                        let now = flags & bit != 0;
                        let before = self.mods & bit != 0;
                        if now != before {
                            self.keys[k as usize] = now;
                            self.events.push(if now { Event::KeyDown(k, false) } else { Event::KeyUp(k) });
                        }
                    }
                    self.mods = flags;
                }
                22 => {
                    let dy = msg!(f64; ev, "scrollingDeltaY");
                    let precise = msg!(bool; ev, "hasPreciseScrollingDeltas");
                    let d = if precise { dy / 12.0 } else { dy.signum() * dy.abs().max(1.0) };
                    if d != 0.0 {
                        self.events.push(Event::Scroll(d as f32));
                    }
                }
                _ => {}
            }
            if forward {
                msg!((); self.app, "sendEvent:", ev => Id);
            }
        }
        msg!((); self.app, "updateWindows");

        let visible = msg!(bool; self.window, "isVisible");
        if !visible {
            self.should_close = true;
        }
        let key_win = msg!(bool; self.window, "isKeyWindow");
        if !key_win && self.focused {
            // Lost focus: release everything.
            self.keys = [false; 256];
            self.buttons = [false; 3];
            self.mods = 0;
        }
        self.focused = key_win;

        // Mouse position in framebuffer pixels (top-left origin)
        let p = msg!(NSPoint; self.window, "mouseLocationOutsideOfEventStream");
        let bounds = msg_rect(self.view, "bounds");
        self.mouse_x = p.x as f32 * self.scale;
        self.mouse_y = (bounds.size.h - p.y) as f32 * self.scale;
        self.update_size();
    }

    fn event_in_view(&self, ev: Id) -> bool {
        let p = msg!(NSPoint; ev, "locationInWindow");
        let bounds = msg_rect(self.view, "bounds");
        p.x >= 0.0 && p.y >= 0.0 && p.x <= bounds.size.w && p.y <= bounds.size.h
    }

    pub fn swap(&self) {
        msg!((); self.ctx, "flushBuffer");
    }

    pub fn is_mouse_locked(&self) -> bool {
        self.mouse_locked
    }

    pub fn set_mouse_locked(&mut self, lock: bool) {
        if lock == self.mouse_locked {
            return;
        }
        self.mouse_locked = lock;
        unsafe {
            if lock {
                self.warp_to_center();
                CGAssociateMouseAndMouseCursorPosition(0);
                if !self.cursor_hidden {
                    msg!((); class("NSCursor"), "hide");
                    self.cursor_hidden = true;
                }
            } else {
                CGAssociateMouseAndMouseCursorPosition(1);
                self.warp_to_center();
                if self.cursor_hidden {
                    msg!((); class("NSCursor"), "unhide");
                    self.cursor_hidden = false;
                }
            }
        }
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
    }

    fn warp_to_center(&self) {
        let frame = msg_rect(self.window, "frame");
        let content = msg_rect_arg(self.window, "contentRectForFrameRect:", frame);
        let screens = msg!(Id; class("NSScreen"), "screens");
        let primary = msg!(Id; screens, "objectAtIndex:", 0u64 => u64);
        let sf = msg_rect(primary, "frame");
        let cx = content.origin.x + content.size.w / 2.0;
        let cy = content.origin.y + content.size.h / 2.0;
        unsafe {
            CGWarpMouseCursorPosition(NSPoint { x: cx, y: sf.size.h - cy });
        }
    }

    pub fn toggle_fullscreen(&self) {
        msg!((); self.window, "toggleFullScreen:", NIL => Id);
    }

    pub fn set_title(&self, title: &str) {
        msg!((); self.window, "setTitle:", nsstring(title) => Id);
    }

    pub fn key(&self, k: u16) -> bool {
        self.keys[k as usize]
    }
}

/// Raw pointer to keep `c_void` import used on all targets.
pub fn _unused(_: *const c_void) {}
