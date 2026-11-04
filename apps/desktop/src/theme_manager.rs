use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Custom theme configuration that can be saved/loaded from ~/.config/rustcord/theme.json
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CustomTheme {
	/// Optional friendly theme name
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// Base preset to inherit: "default", "smooth", "pixeled", "oled", "nord", "dracula", "catppuccin", "solarized", "slate", etc.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub preset: Option<String>,
	/// Custom widget corner radius in logical pixels (e.g. 0 for pixeled, 12 for smooth, 6 for default)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub radius: Option<u8>,
	/// Custom window & menu corner radius in logical pixels
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub window_radius: Option<u8>,
	/// Force square avatars and server icons (true = sharp square, false = circular/squircle)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub square_avatars: Option<bool>,
	/// Enable retro monospace pixel font for the whole UI
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pixel_font: Option<bool>,
	/// Accent color in hex format (e.g. "#00ff66", "#7aa2f7", "#5865f2")
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub accent: Option<String>,
	/// Color token overrides:
	/// "base", "sidebar", "chat", "raised", "hover", "selected", "border",
	/// "text_strong", "text", "muted", "link", "accent", "accent_text",
	/// "positive", "warning", "danger", "mention_bg", "mention_text"
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub colors: BTreeMap<String, String>,
}

impl CustomTheme {
	/// Convert to an internal extensions::Theme structure to apply to the egui context.
	pub fn to_extension_theme(&self) -> Option<extensions::Theme> {
		if self.radius.is_none() && self.window_radius.is_none() && self.colors.is_empty() {
			return None;
		}

		let mut colors = self.colors.clone();
		if let Some(accent) = &self.accent {
			colors
				.entry("accent".to_string())
				.or_insert_with(|| accent.clone());
		}

		let palette = extensions::ThemePalette {
			background: None,
			colors,
			backdrop: None,
		};

		let style = extensions::ThemeStyle {
			widget_radius: self.radius,
			window_radius: self.window_radius,
			menu_radius: self.window_radius,
			..Default::default()
		};

		let theme = extensions::Theme {
			light: palette.clone(),
			dark: palette,
			style,
		};

		if theme.validate().is_ok() {
			Some(theme)
		} else {
			None
		}
	}
}

/// Parse a hex color string like "#00ff66" or "00ff66" or "#fff" into RGB bytes.
pub fn parse_hex_color(hex: &str) -> Option<[u8; 3]> {
	let hex = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
	match hex.len() {
		3 => {
			let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
			let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
			let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
			Some([r, g, b])
		}
		6 => {
			let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
			let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
			let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
			Some([r, g, b])
		}
		_ => None,
	}
}

/// Format RGB bytes to standard hex "#rrggbb".
pub fn format_hex(rgb: [u8; 3]) -> String {
	format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Returns the standard default user config path: ~/.config/rustcord/theme.json
pub fn default_config_path() -> PathBuf {
	if let Some(config_dir) = dirs::config_dir() {
		config_dir.join("rustcord").join("theme.json")
	} else {
		PathBuf::from("theme.json")
	}
}

/// Load theme from a given file path.
pub fn load_from_file(path: &Path) -> Result<CustomTheme, String> {
	let data = std::fs::read_to_string(path)
		.map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
	serde_json::from_str(&data)
		.map_err(|e| format!("Invalid JSON theme in {}: {}", path.display(), e))
}

/// Save theme to a given file path, creating parent directories if needed.
pub fn save_to_file(theme: &CustomTheme, path: &Path) -> Result<(), String> {
	if let Some(parent) = path.parent() {
		let _ = std::fs::create_dir_all(parent);
	}
	let json = serde_json::to_string_pretty(theme)
		.map_err(|e| format!("Failed to serialize theme: {}", e))?;
	std::fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Load theme from active configuration, checking custom file, then default path.
pub fn load_active_theme(theme_file: Option<&Path>) -> Option<CustomTheme> {
	if let Some(path) = theme_file {
		if let Ok(theme) = load_from_file(path) {
			return Some(theme);
		}
	}
	let default_path = default_config_path();
	if default_path.exists() {
		if let Ok(theme) = load_from_file(&default_path) {
			return Some(theme);
		}
	}
	None
}

/// Apply a CustomTheme to ui::design global state.
pub fn apply_theme(theme: &CustomTheme) {
	ui::design::reset_overrides();

	if let Some(preset_name) = &theme.preset {
		if let Some(variant) = ui::design::Variant::from_key(preset_name) {
			ui::design::set_variant(variant);
		}
	}

	if let Some(radius) = theme.radius {
		ui::design::set_corner_radius(Some(radius));
	}

	if let Some(square) = theme.square_avatars {
		ui::design::set_square_avatars(Some(square));
	}

	if let Some(pixel_font) = theme.pixel_font {
		ui::design::set_pixel_font(Some(pixel_font));
	}

	if let Some(accent) = &theme.accent {
		if let Some(rgb) = parse_hex_color(accent) {
			ui::design::set_primary_color(Some(rgb));
		}
	}

	if let Some(ext) = theme.to_extension_theme() {
		ui::design::set_extension_theme(Some(&ext));
	}
}

/// Create a template CustomTheme populated with all available fields for export.
pub fn template_theme() -> CustomTheme {
	let mut colors = BTreeMap::new();
	colors.insert("base".into(), "#08080a".into());
	colors.insert("sidebar".into(), "#0d0e13".into());
	colors.insert("chat".into(), "#121319".into());
	colors.insert("raised".into(), "#181a22".into());
	colors.insert("hover".into(), "#20232e".into());
	colors.insert("selected".into(), "#2a2e3d".into());
	colors.insert("border".into(), "#252834".into());
	colors.insert("text_strong".into(), "#00ff66".into());
	colors.insert("text".into(), "#d8ffd8".into());
	colors.insert("muted".into(), "#609060".into());
	colors.insert("link".into(), "#00e5ff".into());
	colors.insert("accent".into(), "#00ff66".into());
	colors.insert("accent_text".into(), "#000000".into());
	colors.insert("positive".into(), "#00ff66".into());
	colors.insert("warning".into(), "#ffd600".into());
	colors.insert("danger".into(), "#ff1744".into());
	colors.insert("mention_bg".into(), "#00ff6630".into());
	colors.insert("mention_text".into(), "#00ff66".into());

	CustomTheme {
		name: Some("custom-cyberpunk".into()),
		preset: Some("pixeled".into()),
		radius: Some(0),
		window_radius: Some(0),
		square_avatars: Some(true),
		pixel_font: Some(true),
		accent: Some("#00ff66".into()),
		colors,
	}
}

/// Print formatted table of built-in presets and instructions.
pub fn print_theme_list() {
	println!("┌────────────────────────────────────────────────────────────────────────┐");
	println!("│                      RUSTCORD THEME PRESETS                            │");
	println!("├─────────────┬────────┬─────────┬───────────────────────────────────────┤");
	println!("│ Preset      │ Radius │ Accent  │ Description                           │");
	println!("├─────────────┼────────┼─────────┼───────────────────────────────────────┤");
	println!("│ default     │   6px  │ #5865f2 │ Classic Discord dark (blurple)        │");
	println!("│ smooth      │  12px  │ #7aa2f7 │ Sleek modern curves & tokyo blue      │");
	println!("│ pixeled     │   0px  │ #00ff66 │ Retro cyberpunk matrix (sharp edges)  │");
	println!("│ oled        │   8px  │ #a855f7 │ Pitch-black for OLED displays         │");
	println!("│ nord        │   8px  │ #88c0d0 │ Arctic frost blue palette             │");
	println!("│ dracula     │   8px  │ #bd93f9 │ Famous vampire dark & purple palette  │");
	println!("│ catppuccin  │  10px  │ #cba6f7 │ Cozy pastel mocha palette             │");
	println!("│ solarized   │   6px  │ #2aa198 │ Classic solarized cyan dark palette   │");
	println!("│ slate       │   6px  │ #5865f2 │ Lighter blue-grey neutral dark        │");
	println!("│ nightfall   │   6px  │ #5865f2 │ Midnight deep blurple gradient        │");
	println!("│ ember       │   6px  │ #8c2340 │ Crimson moon dark gradient            │");
	println!("│ verdant     │   6px  │ #2b6350 │ Deep emerald forest gradient          │");
	println!("│ afterglow   │   6px  │ #d9663c │ Sunset twilight gradient              │");
	println!("└─────────────┴────────┴─────────┴───────────────────────────────────────┘");

	let current_path = default_config_path();
	if current_path.exists() {
		if let Ok(theme) = load_from_file(&current_path) {
			let preset = theme.preset.unwrap_or_else(|| "default".into());
			let accent = theme.accent.unwrap_or_else(|| "default".into());
			println!("\nActive configuration ({}) :", current_path.display());
			println!(
				"  Preset: {}, Accent: {}, Radius: {:?}",
				preset, accent, theme.radius
			);
		}
	} else {
		println!("\nActive configuration: default (built-in)");
	}

	println!("\nTerminal Commands:");
	println!("  rustcord theme list                    List all presets and active configuration");
	println!(
		"  rustcord theme set <preset>            Switch active preset (e.g. rustcord theme set pixeled)"
	);
	println!(
		"  rustcord theme set --accent <#hex>     Set custom accent color (e.g. rustcord theme set --accent '#00ff66')"
	);
	println!("  rustcord theme set <preset> <#hex>     Set preset and accent in one command");
	println!("  rustcord theme export [path]           Export customizable theme JSON template");
	println!("  rustcord theme current                 Display currently configured theme");
	println!("\nLaunch Flags:");
	println!("  rustcord --theme <preset>              Launch with specified preset");
	println!("  rustcord --accent <#hex>               Launch with custom accent color");
	println!("  rustcord --theme-file <path>           Launch with custom theme JSON file");
}

/// Check CLI arguments and execute theme commands if invoked in terminal mode.
/// Returns Some(exit_code) if a terminal command was handled, or None if GUI app should launch.
pub fn handle_cli(args: &[String]) -> Option<i32> {
	if args.len() < 2 {
		return None;
	}

	if args[1] != "theme" {
		return None;
	}

	let subcmd = args.get(2).map(|s| s.as_str()).unwrap_or("help");

	match subcmd {
		"list" | "ls" => {
			print_theme_list();
			Some(0)
		}
		"current" => {
			let config_path = default_config_path();
			if config_path.exists() {
				match load_from_file(&config_path) {
					Ok(theme) => {
						println!("Config file: {}", config_path.display());
						println!("Preset: {}", theme.preset.as_deref().unwrap_or("default"));
						println!("Accent: {}", theme.accent.as_deref().unwrap_or("default"));
						if let Some(r) = theme.radius {
							println!("Radius: {}px", r);
						}
						if let Some(sq) = theme.square_avatars {
							println!("Square Avatars: {}", sq);
						}
						if let Some(pf) = theme.pixel_font {
							println!("Pixel Font: {}", pf);
						}
					}
					Err(e) => {
						eprintln!("Error reading {}: {}", config_path.display(), e);
						return Some(1);
					}
				}
			} else {
				println!("No custom configuration found. Rustcord is using the default preset.");
				println!("Configuration path: {}", config_path.display());
			}
			Some(0)
		}
		"set" => {
			if args.len() < 4 {
				eprintln!(
					"Usage: rustcord theme set <preset> [--accent <#hex>] [--radius <px>] [--square-avatars [true|false]] [--pixel-font [true|false]]"
				);
				eprintln!("       rustcord theme set --accent <#hex>");
				eprintln!("Run 'rustcord theme list' for available presets.");
				return Some(1);
			}

			let config_path = default_config_path();
			let mut theme = load_from_file(&config_path).unwrap_or_default();

			let mut idx = 3;
			while idx < args.len() {
				let arg = &args[idx];
				if arg == "--accent" {
					if let Some(hex) = args.get(idx + 1) {
						if parse_hex_color(hex).is_some() {
							theme.accent = Some(hex.clone());
							idx += 2;
							continue;
						} else {
							eprintln!(
								"Error: Invalid hex color format '{}'. Example: '#00ff66' or '#7aa2f7'",
								hex
							);
							return Some(1);
						}
					} else {
						eprintln!("Error: Missing value for --accent");
						return Some(1);
					}
				} else if arg == "--radius" {
					if let Some(val) = args.get(idx + 1).and_then(|s| s.parse::<u8>().ok()) {
						theme.radius = Some(val);
						idx += 2;
						continue;
					} else {
						eprintln!("Error: Expected a number 0..=32 for --radius");
						return Some(1);
					}
				} else if arg == "--square-avatars" {
					if let Some(next) = args.get(idx + 1) {
						if let Ok(b) = next.parse::<bool>() {
							theme.square_avatars = Some(b);
							idx += 2;
							continue;
						}
					}
					theme.square_avatars = Some(true);
					idx += 1;
				} else if arg == "--pixel-font" {
					if let Some(next) = args.get(idx + 1) {
						if let Ok(b) = next.parse::<bool>() {
							theme.pixel_font = Some(b);
							idx += 2;
							continue;
						}
					}
					theme.pixel_font = Some(true);
					idx += 1;
				} else if parse_hex_color(arg).is_some() {
					theme.accent = Some(arg.clone());
					idx += 1;
				} else {
					if let Some(variant) = ui::design::Variant::from_key(arg) {
						theme.preset = Some(variant.key().to_string());
						theme.radius = None;
						theme.square_avatars = None;
						theme.pixel_font = None;
					} else {
						eprintln!(
							"Error: Unknown theme preset '{}'. Run 'rustcord theme list' to view available presets.",
							arg
						);
						return Some(1);
					}
					idx += 1;
				}
			}

			if let Err(e) = save_to_file(&theme, &config_path) {
				eprintln!("Failed to save theme: {}", e);
				return Some(1);
			}

			println!(
				"✓ Successfully updated theme settings in {}",
				config_path.display()
			);
			if let Some(p) = &theme.preset {
				println!("  Preset: {}", p);
			}
			if let Some(a) = &theme.accent {
				println!("  Accent: {}", a);
			}
			if let Some(r) = theme.radius {
				println!("  Radius: {}px", r);
			}
			if let Some(sq) = theme.square_avatars {
				println!("  Square Avatars: {}", sq);
			}
			if let Some(pf) = theme.pixel_font {
				println!("  Pixel Font: {}", pf);
			}
			Some(0)
		}
		"export" => {
			let target_path = args
				.get(3)
				.map(PathBuf::from)
				.unwrap_or_else(|| PathBuf::from("theme.json"));
			let template = template_theme();
			if let Err(e) = save_to_file(&template, &target_path) {
				eprintln!("Error exporting theme template: {}", e);
				return Some(1);
			}
			println!(
				"✓ Successfully exported theme template to '{}'",
				target_path.display()
			);
			println!(
				"  You can edit this file and load it with: rustcord --theme-file {}",
				target_path.display()
			);
			Some(0)
		}
		"help" | "--help" | "-h" => {
			print_theme_list();
			Some(0)
		}
		other => {
			eprintln!(
				"Unknown command 'rustcord theme {}'. Run 'rustcord theme list' or 'rustcord theme help'.",
				other
			);
			Some(1)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hex_color_parsing() {
		assert_eq!(parse_hex_color("#ff0000"), Some([255, 0, 0]));
		assert_eq!(parse_hex_color("00ff00"), Some([0, 255, 0]));
		assert_eq!(parse_hex_color("#00f"), Some([0, 0, 255]));
		assert_eq!(format_hex([0, 255, 102]), "#00ff66");
	}

	#[test]
	fn test_custom_theme_serialization() {
		let theme = CustomTheme {
			name: Some("test".into()),
			preset: Some("pixeled".into()),
			radius: Some(0),
			window_radius: Some(0),
			square_avatars: Some(true),
			pixel_font: Some(true),
			accent: Some("#00ff66".into()),
			colors: Default::default(),
		};

		let json = serde_json::to_string(&theme).unwrap();
		let deserialized: CustomTheme = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized.preset.as_deref(), Some("pixeled"));
		assert_eq!(deserialized.radius, Some(0));
		assert_eq!(deserialized.square_avatars, Some(true));
		assert_eq!(deserialized.pixel_font, Some(true));
	}

	#[test]
	fn test_apply_theme() {
		let theme = CustomTheme {
			name: None,
			preset: Some("pixeled".into()),
			radius: Some(0),
			window_radius: None,
			square_avatars: Some(true),
			pixel_font: Some(true),
			accent: Some("#00ff66".into()),
			colors: Default::default(),
		};
		apply_theme(&theme);
		assert_eq!(ui::design::variant(), ui::design::Variant::Pixeled);
		assert_eq!(ui::design::effective_widget_radius(), 0);
		assert_eq!(ui::design::avatar_radius(40.0), 0);
		assert!(ui::design::is_pixel_font());
		assert_eq!(ui::design::primary_color(), Some([0, 255, 102]));

		// Reset to default
		let reset = CustomTheme {
			name: None,
			preset: Some("default".into()),
			radius: Some(6),
			window_radius: None,
			square_avatars: Some(false),
			pixel_font: Some(false),
			accent: None,
			colors: Default::default(),
		};
		apply_theme(&reset);
		assert_eq!(ui::design::variant(), ui::design::Variant::Standard);
		assert_eq!(ui::design::effective_widget_radius(), 6);
		assert_eq!(ui::design::avatar_radius(40.0), 20);
		assert!(!ui::design::is_pixel_font());
	}
}
