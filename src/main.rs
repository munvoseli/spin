use std::io::{Read, Write};
use std::fs::File;

mod feed;

fn serve_html(mut stream: std::net::TcpStream, fal: &str) {
	let mut v = Vec::<u8>::new();
	let mut file = File::open(fal).unwrap();
	file.read_to_end(&mut v).unwrap();
	let mut nv = Vec::<u8>::new();
	let mut n = v.len();
	loop {
		if n == 0 { break; }
		nv.push((n % 10) as u8);
		n /= 10;
	}
	let mut i = nv.len();
	match stream.write
	(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: ") {
	Err(_) => { return; }
	_ => {}
	}
	loop {
		if i == 0 { break; }
		i -= 1;
		if stream.write(&nv[i..i+1]).is_err() { return; }
	}
	if stream.write(b"\r\n\r\n").is_err() { return; }
	if stream.write(&v).is_err() { return; }
}

fn split_path(v: &[u8]) -> Vec<Vec<u8>> {
	let mut wb: Vec<Vec<u8>> = Vec::new();
	let mut i = 0;
	loop {
		if i >= v.len() { break; }
		let mut h = Vec::<u8>::new();
		loop {
			if i >= v.len() || v[i] == 0x2f { break; }
			h.push(v[i]);
			i += 1;
		}
		if h.len() > 0 { wb.push(h); }
		i += 1;
	}
	wb
}

fn handle_client(mut stream: std::net::TcpStream) {
	let mut data = [0 as u8; 50];
	let mut v = Vec::<u8>::new();
	loop {
		match stream.read(&mut data) {
		Ok(0) => {},
		Ok(n) => {
			v.extend_from_slice(&data[0..n]);
			if v.ends_with(&[13,10,13,10]) {
				println!("booi");
				break;
			}
		},
		Err(_) => {
			println!("boi");
			break;
		}
		}
	}
	let mut i = 0;
	loop {
		if v[i] == 32 { break; }
		i += 1;
	} // i on space before path
	let mut j = i + 1;
	loop {
		if v[j] == 32 { break; }
		j += 1;
	} // j on space after path
	let method = &v[0..i];
	let path = &v[i+1..j];
	let s = std::str::from_utf8(method).unwrap();
	let t = std::str::from_utf8(path).unwrap();
	println!("Request {} {}", s, t);
	let path = split_path(&path);
	println!("Parsed path with {} terms", path.len());
	if path.len() == 0 {
		serve_html(stream, "index.html");
	} else if path[0] == [0x66, 0x65, 0x65, 0x64] {
		crate::feed::update_feed();
		serve_html(stream, "feed.html");
	} else {
		serve_html(stream, "index.html");
	}
//	let r = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 0\r\n\r\n";
//	stream.write(r).unwrap();
//	stream.shutdown(std::net::Shutdown::Both).unwrap();
//	stream.write(&v).unwrap();
}

fn main() {
	let listener = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
	for stream in listener.incoming() {
		println!("got new stream");
		handle_client(stream.unwrap());
	}
}
