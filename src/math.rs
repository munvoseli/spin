use std::fs::File;
//use std::io::{Read, Write};
use std::io::Write;

//struct Glyph<'a> {
//	sym: &'a str,
//	ax: f32,
//	ay: f32,
//	svg: &'a str
//}
//const glyphs: Vec<Glyph> = vec!(
//	Glyph { sym: "+", ax: 5.0, ay: 5.0, svg: "" }
//);
//
//struct Gloper {
//	gl: usize,
//
//}

struct Glynst {
	x: f32,
	y: f32,
	svg: String
}

struct Glyphbox {
	x0: f32,
	x1: f32,
	y0: f32,
	y1: f32,
	g: Vec<Glynst>
}

fn maxft(a: f32, b: f32, c: f32) -> f32 {
	if a > b {
		return if a > c { a } else { c };
	} else {
		return if b > c { b } else { c };
	}
}
fn minft(a: f32, b: f32, c: f32) -> f32 {
	if a < b {
		return if a < c { a } else { c };
	} else {
		return if b < c { b } else { c };
	}
}
fn maxdt(a: f32, b: f32) -> f32 {
	if a < b { b } else { a }
}

impl Glyphbox {
	fn from_pathwh(s: &str, w: f32, h: f32) -> Self {
		let s: String = format!("<path fill-rule=\"evenodd\" d=\"{}\"/>", s);
		Self { x0: -w/2.0, x1: w/2.0,
			y0: -h/2.0, y1: h/2.0,
			g: vec!( Glynst { x: 0.0, y: 0.0, svg: s } )
		}
	}
	fn hdop(stack: &mut Vec<Glyphbox>, svgp: &str, opax: f32, opay: f32) {
		if stack.len() < 2 {
			println!("not enough stack");
		}
		let mut sa = stack.pop().unwrap(); // sa on right
		let x = sa.x0 - opax - 1.0;
		sa.x1 -= x;
		sa.x0 -= x;
		for gly in &mut sa.g {
			gly.x -= x;
		}
		let mut sb = stack.pop().unwrap(); // sb on left
		let x = sb.x1 + opax + 1.0;
		sb.x1 -= x;
		sb.x0 -= x;
		for gly in &mut sb.g {
			gly.x -= x;
		}
		sa.g.append(&mut sb.g);
		sa.x0 = sb.x0;
		sa.y0 = minft(sa.y0, sb.y0, -opay);
		sa.y1 = maxft(sa.y1, sb.y1, opay);
		let s: String = format!("<path fill-rule=\"evenodd\" d=\"{}\"/>", svgp);
		sa.g.push(Glynst { x: 0.0, y: 0.0, svg: s });
		stack.push(sa);
	}
	fn hline(stack: &mut Vec<Glyphbox>) {
		// sa on bottom
		if stack.len() < 2 {
			println!("not enough stack");
		}
		let mut sa = stack.pop().unwrap();
		let dy = sa.y0 - 4.0;
		let dx = (sa.x0 + sa.x1) / 2.0;
		sa.y0 -= dy; sa.y1 -= dy;
		for gly in &mut sa.g { gly.y -= dy; gly.x -= dx; }

		let mut sb = stack.pop().unwrap();
		let dy = sb.y1 + 4.0;
		let dx = (sb.x0 + sb.x1) / 2.0;
		sb.y0 -= dy; sb.y1 -= dy;
		for gly in &mut sb.g { gly.y -= dy; gly.x -= dx; }

		let aw = maxdt(sa.x1 - sa.x0, sb.x1 - sb.x0) / 2.0 + 4.0;
		sa.g.append(&mut sb.g);
		sa.x0 = -aw;
		sa.x1 = aw;
		sa.y0 = sb.y0;
		let s: String = format!("<path fill-rule=\"evenodd\" d=\"M {} -0.5 v 1 H {} v -1 z\"/>", -aw, aw);
		sa.g.push(Glynst { x: 0.0, y: 0.0, svg: s });
		stack.push(sa);
	}
}

// each word corresponds to a stack operation and glyph additions
// 0..9 will push a glyphbox containing a single glyph onto the stack
// +-/* will get two glyphboxes on the stack and combine them and glyph addition

fn get_glyph_set(source: &[u8]) -> Glyphbox {
	let mut stack: Vec<Glyphbox> = Vec::new();
	let mut i = 0;
	loop {
		while i < source.len() && source[i] <= 0x20 { i += 1; }
		if i >= source.len() { break; }
		// i is now at beginning of word
		let mut j = i + 1;
		while j < source.len() && source[j] > 0x20 { j += 1; }
		// j is at space at end of word
		let w = &source[i..j];
		match w {
			[0x30] => {
				let gb = Glyphbox::from_pathwh("M 0 -5 Q 3 -5, 3 0 T 0 5 T -3 0 T 0 -5  M 0 -4 Q 2 -4, 2 0 T 0 4 T -2 0 T 0 -4", 6.0, 10.0);
				stack.push(gb);
			},
			[0x31] => {
				let gb = Glyphbox::from_pathwh("M -1.5 -4 q 2 0, 2 -1 h 1 v 10 h -1 v -8 h -2 z", 3.0, 10.0);
				stack.push(gb);
			},
			[0x2b] => {
				Glyphbox::hdop(&mut stack, "M -4 -0.5 h 3.5 v -3.5 h 1 v 3.5 h 3.5 v 1 h -3.5 v 3.5 h -1 v -3.5 h -3.5 z", 4.0, 4.0);
			},
			[0x2f] => {
				Glyphbox::hline(&mut stack);
			},
			[..] => {
				println!("unknown operation");
			}
		}
		i = j;
	}
	stack.swap_remove(0)
}

pub fn write_svg(file: &mut File, data: &[u8]) {
	let gbox = get_glyph_set(data);
	file.write(b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"").unwrap();
	file.write(format!("{} {} {} {}\" width=\"{}\">", gbox.x0, gbox.y0, gbox.x1-gbox.x0, gbox.y1-gbox.y0, (gbox.x1-gbox.x0)*2.0).as_bytes()).unwrap();
	for gly in &gbox.g {
		file.write(format!("<g transform=\"translate({}, {})\">", gly.x, gly.y).as_bytes()).unwrap();
		file.write(gly.svg.as_bytes()).unwrap();
		file.write(b"</g>").unwrap();
	}
	file.write(b"</svg>").unwrap();
}


pub fn update_math() {
	let mut file = File::create("math.html").unwrap();
	file.write(b"<!DOCTYPE html><html><body>math<br>").unwrap();
	write_svg(&mut file, b"0 1 1 1 1 + + / + 1 /");
	file.write(b"</body></html>").unwrap();
}
