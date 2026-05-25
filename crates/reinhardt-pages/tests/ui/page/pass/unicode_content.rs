//! page! macro with Unicode content
//!
//! This test verifies that the page! macro correctly handles Unicode characters
//! including Japanese, Chinese, Korean, Arabic, Emoji, and special symbols.

use reinhardt_pages::page;

fn main() {
	// Japanese (Hiragana, Katakana, Kanji)
	let _japanese = page!(|| {
		div {
			p { "こんにちは世界" }
			p { "カタカナテスト" }
			p { "日本語のテキスト" }
		}
	});

	// Chinese (Simplified and Traditional)
	let _chinese = page!(|| {
		div {
			p { "你好世界" }
			p { "繁體中文" }
		}
	});

	// Korean (Hangul)
	let _korean = page!(|| {
		div {
			p { "안녕하세요" }
			p { "한글 테스트" }
		}
	});

	// Arabic (RTL text)
	let _arabic = page!(|| {
		div {
			p { "مرحبا بالعالم" }
			p { "النص العربي" }
		}
	});

	// Emoji
	let _emoji = page!(|| {
		div {
			p { "😀 😃 😄 😁 😆" }
			p { "🎉 🎊 🎈 🎁 🎀" }
			p { "❤️ 💛 💚 💙 💜" }
			p { "🌟 ⭐ ✨ 💫 ⚡" }
		}
	});

	// Mathematical and special symbols
	let _symbols = page!(|| {
		div {
			p { "∑ ∏ ∫ ∂ ∇" }
			p { "α β γ δ ε" }
			p { "Α Β Γ Δ Ε" }
			p { "← → ↑ ↓ ↔" }
			p { "© ® ™ ℗ ℠" }
		}
	});

	// Mixed content with attributes
	let _mixed = page!(|| {
		div {
			img {
				src: "/emoji.png",
				alt: "絵文字の画像 🎨",
			}
			a {
				href: "/日本語/パス",
				title: "日本語のタイトル",
				"リンクテキスト 🔗"
			}
			button {
				aria_label: "クリックしてください",
				@click: |_| {},
				"ボタン 🔘"
			}
		}
	});

	// Complex multilingual content
	let _multilingual = page!(|| {
		article {
			h1 { "🌍 Multilingual Support" }
			section {
				h2 { "日本語セクション" }
				p { "これは日本語のテキストです。" }
			}
			section {
				h2 { "中文部分" }
				p { "这是中文文本。" }
			}
			section {
				h2 { "한국어 섹션" }
				p { "이것은 한국어 텍스트입니다." }
			}
			section {
				h2 { "القسم العربي" }
				p { "هذا نص عربي." }
			}
			footer { "© 2024 🌐 Global Inc." }
		}
	});

	// Zero-width and combining characters
	let _special_chars = page!(|| {
		div {
			p { "é" }
			p { "à" }
			p { "test​word" }
		}
	});

	// Long Unicode text
	let _long_text = page!(|| {
		div {
			p { "吾輩は猫である。名前はまだ無い。どこで生れたかとんと見当がつかぬ。何でも薄暗いじめじめした所でニャーニャー泣いていた事だけは記憶している。吾輩はここで始めて人間というものを見た。🐱" }
		}
	});
}
