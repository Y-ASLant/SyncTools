use std::env;

fn main() {
    // 设置环境变量跳过图标生成
    env::set_var("TAURI_BUNDLE_SKIP", "true");
    env::set_var("TAURI_SKIP_ASSET_HASH", "true");

    tauri_build::build();
}
