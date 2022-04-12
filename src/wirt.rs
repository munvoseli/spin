use std::fs::File;
use std::io::{Read, Write};

pub fn serve_file(stream: &mut std::net::TcpStream, fal: &str, contype: &[u8]) {
	let mut v = Vec::<u8>::new();
	let mut file = File::open(fal).unwrap();
	file.read_to_end(&mut v).unwrap();
	let mut nv = Vec::<u8>::new();
	let mut n = v.len();
	loop {
		if n == 0 { break; }
		nv.push(((n % 10) as u8) ^ 0x30);
		n /= 10;
	}
	let mut i = nv.len();
	if stream.write(b"HTTP/1.1 200 OK\r\nContent-Type: ").is_err()
	|| stream.write(contype).is_err()
	|| stream.write(b"\r\nContent-Length: ").is_err()
	{ return; }
	loop {
		if i == 0 { break; }
		i -= 1;
		if stream.write(&nv[i..i+1]).is_err() { return; }
	}
	if stream.write(b"\r\n\r\n").is_err() { return; }
	if stream.write(&v).is_err() { return; }
}
pub fn serve_html(stream: &mut std::net::TcpStream, fal: &str) {
	serve_file(stream, fal, b"text/html");
}

pub fn html_file(outfile: &mut File, f: &mut File) {
	let mut buf: [u8; 50] = [0; 50];
	loop {
		let n = f.read(&mut buf).unwrap();
		if n == 0 { break; }
		outfile.write(&buf[0..n]).unwrap();
	}
}

pub fn html_template(outfile: &mut File, s: &str) {
	let mut f = File::open(s).unwrap();
	html_file(outfile, &mut f);
}
