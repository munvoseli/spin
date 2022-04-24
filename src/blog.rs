use std::fs::File;
use std::io::{Read, Write};
use tokio::net::TcpStream;
//use tokio::io::AsyncWriteExt;
use crate::wirt;

fn parse_write(outfile: &mut File, buf: &[u8]) {
//	println!("writing paragraph {}", buf.len());
	if buf.len() > 2 && buf[0..3] == [0x6d, 0x74, 0x0a] {
		crate::math::write_svg(outfile, &buf[3..]);
	} else if buf.len() > 2 && buf[0..3] == [0x6c, 0x6e, 0x0a] {
		outfile.write(b"<p><a href=\"").unwrap();
		let mut j = 3;
		while j < buf.len() && buf[j] >= 0x20 { j += 1; }
		outfile.write(&buf[3..j]).unwrap();
		outfile.write(b"\">").unwrap();
		if j != buf.len() {
			outfile.write(&buf[j..]).unwrap();
		} else {
			outfile.write(&buf[3..j]).unwrap();
		}
		outfile.write(b"</a></p>").unwrap();
	} else if buf.len() == 2 && buf[0..2] == [0x2d, 0x2d] {
		outfile.write(b"<hr/>").unwrap();
	} else if buf.len() > 2 && buf[0..3] == [0x69, 0x6d, 0x0a] {
		outfile.write(b"<img src=\"").unwrap();
		let mut j = 3;
		while j < buf.len() && buf[j] >= 0x20 { j += 1; }
		outfile.write(&buf[3..j]).unwrap();
		if j != buf.len() {
			j += 1;
			outfile.write(b"\" alt=\"").unwrap();
			outfile.write(&buf[j..]).unwrap();
			outfile.write(b"\" title=\"").unwrap();
			outfile.write(&buf[j..]).unwrap();
		}
		outfile.write(b"\"/>").unwrap();
	} else {
		outfile.write(b"<p>").unwrap();
		// line loop
		let mut i = 0;
		loop {
			let j = i;
			if i >= buf.len() { break; }
			while i < buf.len() && buf[i] != 10 { i += 1; }
			outfile.write(&buf[j..i]).unwrap();
			if i != buf.len() {
				outfile.write(b"<br/>").unwrap();
			}
			i += 1;
		}
		outfile.write(b"</p>").unwrap();
	}
}


pub async fn update_blog(slug: &str, slugb: &[u8]) -> bool {
	let fname = format!("blogsrc/{}.txt", slug);
	let mdfile = File::open(fname);
	if mdfile.is_err() {
		return false;
	}
	let mut mdfile = mdfile.unwrap();
	let mut buf = Vec::new();
	mdfile.read_to_end(&mut buf).unwrap();
	buf.push(10);
	let fname = format!("blogdst/{}.html", slug);
	let mut outfile = File::create(fname).unwrap();
	wirt::html_template(&mut outfile, "a.html");
	outfile.write(b"<p><a href=\"/edit/").unwrap();
	outfile.write(slugb).unwrap();
	outfile.write(b"\">edit</a></p>").unwrap();
	let mut i = 0;
	loop {
		while i < buf.len() && buf[i] <= 0x20 { i += 1; }
		if i >= buf.len() { break; }
		let mut j = i + 2;
		while j < buf.len()-1 && (buf[j] != 10 || buf[j+1] != 10) { j += 1; }
		parse_write(&mut outfile, &buf[i..j]);
		i = j;
	}
	wirt::html_template(&mut outfile, "z.html");
	true
}

pub async fn serve_blog(stream: &mut TcpStream, slugb: &[u8]) {
	// if the html does not exist or is outdated, make new html
	// right now, just if dne
	let slug = std::str::from_utf8(slugb).unwrap();
	let fname = format!("blogdst/{}.html", slug);
	match File::open(&fname) {
		Ok(_file) => {
			wirt::serve_html(stream, &fname).await;
		},
		Err(..) => {
			if update_blog(&slug, slugb).await {
				wirt::serve_html(stream, &fname).await;
			} else {
				serve_edit(stream, slugb).await;
			}
		}
	}
}

fn asciitohex(x: u8) -> u8 {
	if x < 0x40 {
		x ^ 0x30
	} else {
		(x & 15) + 9
	}
}

pub async fn post(stream: &mut TcpStream, slugb: &[u8], data: &[u8]) {
//	println!("post method boi");
	let mut nd = Vec::<u8>::new();
	let mut i = 7;
	loop {
		if i >= data.len() { break; }
		if i + 2 < data.len() && data[i] == 0x25 {
			i += 1;
			let mut c = asciitohex(data[i]) << 4;
			i += 1;
			c |= asciitohex(data[i]);
			if c != 13 { nd.push(c); }
		} else {
			nd.push(if data[i] == 0x2b { 0x20 } else { data[i] });
		}
		i += 1;
	}
//	println!("{}", std::str::from_utf8(&nd).unwrap());
	let slug = std::str::from_utf8(slugb).unwrap();
	let fname = format!("blogsrc/{}.txt", slug);
	let mut outfile = File::create(&fname).unwrap();
	outfile.write_all(&nd).unwrap();
	if update_blog(&slug, slugb).await {
		let fname = format!("blogdst/{}.html", slug);
		wirt::serve_html(stream, &fname).await;
	} else {
		wirt::serve_html(stream, "index.html").await;
	}
}

pub async fn serve_edit(stream: &mut TcpStream, slug: &[u8]) {
	let fname = format!("blogsrc/{}.txt", std::str::from_utf8(slug).unwrap());
	let mut outfile = File::create("edit.html").unwrap();
	wirt::html_template(&mut outfile, "a.html");
	outfile.write(b"<form action=\"/blog/").unwrap();
	outfile.write(slug).unwrap();
	outfile.write(b"\" method=\"POST\"><textarea name=\"gerald\">").unwrap();
	match File::open(&fname) {
		Ok(mut file) => {
			wirt::html_file(&mut outfile, &mut file);
		},
		Err(..) => {
			
		}
	}
	outfile.write(b"</textarea><input type=\"submit\"></form>").unwrap();
	wirt::html_template(&mut outfile, "z.html");
	wirt::serve_html(stream, "edit.html").await;
}
