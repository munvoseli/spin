//use std::io::Read;
//use std::fs::File;
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;

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

struct Httpboi<'a> {
	v: &'a [u8],
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

async fn handle_http(stream: &mut TcpStream) {
	let mut data = [0 as u8; 50];
	let mut v = Vec::<u8>::new();
//	let mut headers = Vec::<(usize, usize, usize, usize)>::new();
	let mut conlen: usize = 0;
	let mut icrlf: usize = 0;
	let mut dcrlf = false;
	let now = std::time::Instant::now();
	loop {
		match stream.read(&mut data).await {
		Ok(0) => {
//			println!("secsz {} {}", now.elapsed().as_secs(), i);
			if now.elapsed().as_secs() > 10 {
				println!("        timed out {}", now.elapsed().as_secs());
				return; // pizza with left beef
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
//					println!("{}", std::str::from_utf8(&v).unwrap());
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
					handle_request(stream, Httpboi {
						v: &v[0..icrlf + 4 + conlen],
						imd: (0, i),
						ipt: (i+1, j),
						icont: icrlf + 4
					}).await;
					//dcrlf = false;
					//v.drain(0..icrlf + 4 + conlen);
					//icrlf = 0;
					//conlen = 0;
//					match stream.shutdown(Shutdown::Both) {
//					Ok(..) => {
//						println!("shut down stream");
//					},
//					Err(h) => {
//						println!("couldn't shut down stream: {}", h);
//					}
//					}
					return;
				}
			}
		},
		Err(_) => {
			println!("boi");
			break;
		}
		}
	}
}

async fn handle_request<'a>(stream: &mut TcpStream, hb: Httpboi<'a>) {
	println!("################################################");
	println!("{}", std::str::from_utf8(&hb.v).unwrap());
	let method = &hb.v[hb.imd.0..hb.imd.1];
	let path = &hb.v[hb.ipt.0..hb.ipt.1];
	let s = std::str::from_utf8(method).unwrap();
	let t = std::str::from_utf8(path).unwrap();
	println!("Request {} {}", s, t);
	let path = split_path(&path);
	println!("Parsed path with {} terms", path.len());
	if path.len() == 0 {
		wirt::serve_html(stream, "index.html").await;
	} else if path[0] == [0x66, 0x65, 0x65, 0x64] {
		feed::update_feed().await;
		wirt::serve_html(stream, "feed.html").await;
	} else if path[0] == [0x6d, 0x61, 0x74, 0x68] {
		math::update_math();
		wirt::serve_html(stream, "math.html").await;
	} else if path[0] == [0x62, 0x6c, 0x6f, 0x67] {
		if path.len() >= 2 {
			if method == [0x47, 0x45, 0x54] {
				crate::blog::serve_blog(stream, &path[1]).await;
			} else {
				crate::blog::post(stream, &path[1], &hb.v[hb.icont..]).await;
			}
		} else {
			wirt::serve_html(stream, "index.html").await;
		}
	} else if path[0] == [0x65, 0x64, 0x69, 0x74] {
		if path.len() >= 2 {
			crate::blog::serve_edit(stream, &path[1]).await;
		} else {
			wirt::serve_html(stream, "index.html").await;
		}
	} else if path[0] == b"style.css" {
		wirt::serve_file(stream, "style.css", b"text/css").await;
	} else if path[0] == b"favicon.ico" {
		wirt::serve_file(stream, "favicon.ico", b"image/x-icon").await;
	} else {
		wirt::serve_html(stream, "index.html").await;
	}
}

async fn handle_client(mut stream: TcpStream) {
	println!("connection from {}", stream.peer_addr().unwrap());
	handle_http(&mut stream).await;
}

fn main() {
	tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
	let listener = tokio::net::TcpListener::bind("127.0.0.1:2627").await.unwrap();
	loop {
		let (stream, _) = listener.accept().await.unwrap();
		println!("got new stream");
		handle_client(stream).await;
		println!("finished stream");
	}
	});
}
