// 纯文本模式的图标定义
pub struct Icons {
    pub model: &'static str,
    pub directory: &'static str,
    pub git: &'static str,
    pub usage: &'static str,
}

pub const NERD_ICONS: Icons = Icons {
    model: "\u{e26d}",      // 
    directory: "\u{f024b}",  // 󰉋
    git: "\u{f02a2}",        // 󰊢
    usage: "\u{f49b}",       // 
};

pub const EMOJI_ICONS: Icons = Icons {
    model: "🤖",
    directory: "📁",
    git: "🌿",
    usage: "📊",
};

pub const ASCII_ICONS: Icons = Icons {
    model: "[M]",
    directory: "[D]",
    git: "[G]",
    usage: "[U]",
};

pub const NO_ICONS: Icons = Icons {
    model: "",
    directory: "",
    git: "",
    usage: "",
};

pub fn get_icons(mode: &str) -> Icons {
    match mode {
        "emoji" => EMOJI_ICONS,
        "ascii" => ASCII_ICONS,
        "none" => NO_ICONS,
        _ => NERD_ICONS,
    }
}