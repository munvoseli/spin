use std::io::Read;
//use std::fs::File;

mod feed;
mod math;
mod blog;
mod wirt;


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

struct Httpboi {
	v: Vec<u8>,
//	ihs: Vec<(usize, usize, usize, usize)>,
	imd: (usize, usize),
	ipt: (usize, usize),
//	conlen: usize,
	icont: usize
}

fn headboi(v: &[u8], icrlf: usize) -> Vec<(usize, usize, usize, usize)> {
	let mut i = 0;
	let mut h = Vec::<(usize, usize, usize, usize)>::new();
	// skip past GET / ...
	while v[i] != 10 { i += 1; }
	i += 1;
	loop {
		if i >= icrlf { break; }
		// i positioned at beginning
		let mut j = i;
		while j < icrlf && v[j] != 0x3a { j += 1; }
		// j positioned on colon
		let mut k = j + 2;
		while k < icrlf && v[k] != 0xd { k += 1; }
		// k positioned on CR
		h.push((i, j, j + 2, k));
		i = k + 2;
	}
	h
}

fn get_conlen(v: &[u8], h: &[(usize, usize, usize, usize)]) -> usize {
	for header in h {
		if &v[header.0..header.1] == b"Content-Length" {
			let mut n = 0;
			for i in header.2..header.3 {
				n *= 10;
				n += (v[i] ^ 0x30) as usize;
			}
			return n;
		}
	}
	0
}

fn handle_http(stream: &mut std::net::TcpStream) -> Option<Httpboi> {
	let mut data = [0 as u8; 50];
	let mut v = Vec::<u8>::new();
//	let mut headers = Vec::<(usize, usize, usize, usize)>::new();
	let mut conlen: usize = 0;
	let mut icrlf: usize = 0;
	let mut dcrlf = false;
	let now = std::time::Instant::now();
//	let mut i = 0;
	loop {
		match stream.read(&mut data) {
		Ok(0) => {
//			println!("secsz {} {}", now.elapsed().as_secs(), i);
//			i += 1;
			if now.elapsed().as_secs() > 10 {
				println!("timed out {}", now.elapsed().as_secs());
				println!("peer ip: {}", stream.peer_addr().unwrap());
				return None; // pizza with left beef
			}
		},
		Ok(n) => {
//			println!("secsn {}", now.elapsed().as_secs());
			v.extend_from_slice(&data[0..n]);
			if !dcrlf {
				while icrlf < v.len() - 3 {
					if v[icrlf..icrlf+4] == [13,10,13,10] {
						dcrlf = true;
						let headers = headboi(&v, icrlf);
						conlen = get_conlen(&v, &headers);
						break;
					}
					icrlf += 1;
				}
			}
			if dcrlf {
				if v.len() >= icrlf + 4 + conlen {
//					println!("buff: {}", v.len());
//					println!("head: {}", icrlf);
//					println!("cont: {}", conlen);
//					println!("head + cont + 4: {}", conlen + icrlf + 4);
//					println!("booi");
					break;
				}
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
//	let method = &v[0..i];
//	let path = &v[i+1..j];
	Some(Httpboi {
		v: v,
//		ihs: headers,
		imd: (0, i),
		ipt: (i+1, j),
//		conlen: conlen,
		icont: icrlf + 4
	})
}

fn handle_client(mut stream: std::net::TcpStream) {
	println!("connection from {}", stream.peer_addr().unwrap());
	let hb = handle_http(&mut stream);
	if hb.is_none() {
		println!("couldn't handle connection");
		return;
	}
	let hb = hb.unwrap();
	let method = &hb.v[hb.imd.0..hb.imd.1];
	let path = &hb.v[hb.ipt.0..hb.ipt.1];
	let s = std::str::from_utf8(method).unwrap();
	let t = std::str::from_utf8(path).unwrap();
	println!("Request {} {}", s, t);
	let path = split_path(&path);
	println!("Parsed path with {} terms", path.len());
	if path.len() == 0 {
		wirt::serve_html(stream, "index.html");
	} else if path[0] == [0x66, 0x65, 0x65, 0x64] {
		feed::update_feed();
		wirt::serve_html(stream, "feed.html");
	} else if path[0] == [0x6d, 0x61, 0x74, 0x68] {
		math::update_math();
		wirt::serve_html(stream, "math.html");
	} else if path[0] == [0x62, 0x6c, 0x6f, 0x67] {
		if path.len() >= 2 {
			if method == [0x47, 0x45, 0x54] {
				crate::blog::serve_blog(stream, &path[1]);
			} else {
				crate::blog::post(stream, &path[1], &hb.v[hb.icont..]);
			}
		} else {
			wirt::serve_html(stream, "index.html");
		}
	} else if path[0] == [0x65, 0x64, 0x69, 0x74] {
		if path.len() >= 2 {
			crate::blog::serve_edit(stream, &path[1]);
		} else {
			wirt::serve_html(stream, "index.html");
		}
	} else if path[0] == [0x73, 0x74, 0x79, 0x6c, 0x65, 0x2e, 0x63, 0x73, 0x73] {
		wirt::serve_file(stream, "style.css", b"text/css");
	} else if path[0] == b"favicon.ico" {
		wirt::serve_file(stream, "favicon.ico", b"image/x-icon");
	} else {
		wirt::serve_html(stream, "index.html");
	}
//	stream.write(r).unwrap();
//	stream.shutdown(std::net::Shutdown::Both).unwrap();
//	stream.write(&v).unwrap();
}

fn main() {
	let listener = std::net::TcpListener::bind("127.0.0.1:2627").unwrap();
	for stream in listener.incoming() {
		println!("got new stream");
		handle_client(stream.unwrap());
		println!("finished stream");
	}
}
