use crossbeam_channel::Sender;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, RegisterHotKey, VK_SPACE
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

pub fn start_hotkey_thread(tx: Sender<()>) {
    std::thread::spawn(move || unsafe {
        // 🔥 CTRL + SPACE
        let _ = RegisterHotKey(None, 1, MOD_CONTROL, VK_SPACE.0 as u32);

        let mut msg = MSG::default();

        loop {
            if GetMessageW(&mut msg, None, 0, 0).into() {
                if msg.message == WM_HOTKEY {
                    let _ = tx.send(());
                }
            }
        }
    });
}