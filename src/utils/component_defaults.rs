pub struct Color {
    pub name: &'static str,
    pub slug: &'static str,
    pub color: &'static str,
}

pub struct FontSize {
    pub name: &'static str,
    pub slug: &'static str,
    pub size: &'static str,
}

pub const DEFAULT_COLORS: &[Color] = &[
    Color { name: "Black", slug: "black", color: "#000000" },
    Color { name: "Cyan bluish gray", slug: "cyan-bluish-gray", color: "#abb8c3" },
    Color { name: "White", slug: "white", color: "#ffffff" },
    Color { name: "Pale pink", slug: "pale-pink", color: "#f78da7" },
    Color { name: "Vivid red", slug: "vivid-red", color: "#cf2e2e" },
    Color { name: "Luminous vivid orange", slug: "luminous-vivid-orange", color: "#ff6900" },
    Color { name: "Luminous vivid amber", slug: "luminous-vivid-amber", color: "#fcb900" },
    Color { name: "Light green cyan", slug: "light-green-cyan", color: "#7bdcb5" },
    Color { name: "Vivid green cyan", slug: "vivid-green-cyan", color: "#00d084" },
    Color { name: "Pale cyan blue", slug: "pale-cyan-blue", color: "#8ed1fc" },
    Color { name: "Vivid cyan blue", slug: "vivid-cyan-blue", color: "#0693e3" },
    Color { name: "Vivid purple", slug: "vivid-purple", color: "#9b51e0" },
];

pub const DEFAULT_FONT_SIZES: &[FontSize] = &[
    FontSize { name: "Small", slug: "small", size: "13px" },
    FontSize { name: "Medium", slug: "medium", size: "20px" },
    FontSize { name: "Large", slug: "large", size: "36px" },
    FontSize { name: "Extra Large", slug: "x-large", size: "42px" },
];
