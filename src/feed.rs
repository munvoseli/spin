use std::io::{Read, Write};
use std::fs::File;
use std::time::SystemTime;
use std::sync::{Arc, Mutex};

fn beginstrb(h: &[u8], n: &[u8]) -> bool {
	if n.len() > h.len() { return false; }
	for i in 0..n.len() {
		if h[i] != n[i] { return false; }
	}
	return true
}

fn get_hash_for_site(url: &str) -> Option<u32> {
	let boi = reqwest::blocking::get(url);
	match boi {
	Err(_) => {
		return None;
	},
	_ => {}
	}
	let boi = boi.unwrap().bytes().unwrap();
//	let mut boi = boi.into_iter();
	let mut sum: u32 = 0;
	let mut i = 0;
	loop {
		if i >= boi.len() { break; }
//		if let Some(bh) = boi.next() {
		// skip lastBuildDate rss
		if beginstrb(&boi[i..], b"<lastBuildDate") { i += 30; }
		if beginstrb(&boi[i..], b"csrf") { i += 100; }
		if beginstrb(&boi[i..], b"data-cfemail") { i += 100; }
		// don't count digit changes
		// could be milliseconds used to generate the page
		// or page view counter
		// or date
		if boi[i] < 0x30 || boi[i] > 0x39 {
			sum += boi[i] as u32;
		}
		i += 1;
//		} else { break; }
	}
	Some(sum)
}

struct FeedItem {
	lookup: Option<String>,
	href: Option<String>,
	text: Option<String>,
	date: Option<u64>,
	hash: Option<u32>
}
impl FeedItem {
	pub fn new() -> Self {
		Self {
			lookup: None,
			href: None,
			text: None,
			date: None,
			hash: None
		}
	}
}

fn read_feed() -> Vec<FeedItem> {
	let mut file = File::open("feed.txt").unwrap();
	let mut v = Vec::<u8>::new();
	let mut i: usize = 0;
	file.read_to_end(&mut v).unwrap();
	let mut items = Vec::new();
	'boeh: loop { // for entry
		loop {
			if i == v.len() { break 'boeh; }
			if v[i] != 10 { break; }
			i += 1;
		}
		let mut item = FeedItem::new();
		loop {
			if i >= v.len() { break 'boeh; }
			if v[i] == 10 { break; }
			let t: u16 = ((v[i] as u16) << 8) | (v[i + 1] as u16);
			i += 3;
			let j = i; // j is at beginning of entry
			loop {
				if i == v.len() { break; }
				if v[i] == 10 { break; }
				i += 1;
			} // i is on newline or positioned after thing
			let s: String =
			std::str::from_utf8(&v[j..i]).unwrap().into();
			match t {
			0x6c6e => { item.lookup = Some(s); }, // ln
			0x7266 => { item.href = Some(s); }, // rf
			0x7478 => { item.text = Some(s); }, // tx
			0x6474 => { // dt
				item.date = Some(s.parse::<u64>().unwrap());
			},
			0x6873 => { // hs
				item.hash = Some(s.parse::<u32>().unwrap());
			},
			a => { println!("unknown field in feed {:02x}", a); }
			}
			i += 1;
		}
		items.push(item);
	}
	items
}

fn save_feed(items: &Vec<FeedItem>) {
	let mut file = File::create("feed.txt").unwrap();
	for item in items {
		if let Some(h) = &item.lookup {
			file.write(format!("ln {}\n", h).as_bytes()).unwrap();
		}
		if let Some(h) = &item.href {
			file.write(format!("rf {}\n", h).as_bytes()).unwrap();
		}
		if let Some(h) = &item.text {
			file.write(format!("tx {}\n", h).as_bytes()).unwrap();
		}
		if let Some(n) = item.date {
			file.write(format!("dt {}\n", n.to_string()).as_bytes()).unwrap();
		}
		if let Some(n) = item.hash {
			file.write(format!("hs {}\n", n.to_string()).as_bytes()).unwrap();
		}
		file.write(b"\n").unwrap();
	}
}

fn save_feed_html(items: &Vec<FeedItem>) {
	let mut file = File::create("feed.html").unwrap();
	file.write(b"<!DOCTYPE html><html><body>").unwrap();
	let currdate = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH).unwrap()
		.as_secs();
	for item in items {
		let url =
			if let Some(h) = &item.href { h }
			else { item.lookup.as_ref().unwrap() };
		let text =
			if let Some(h) = &item.text { h }
			else if let Some(h) = &item.href { h }
			else { item.lookup.as_ref().unwrap() };
		file.write(
			format!("<br/>{} <a href=\"{}\">{}</a>", currdate - item.date.unwrap(), url, text).as_bytes()
		).unwrap();
		file.write(b"\n").unwrap();
	}
	file.write(b"</body></html>\n").unwrap();
}

fn sort_feed(items: &mut Vec<FeedItem>) {
	// insertion or selection sort, it's prob mostly sorted already
	// also not worried about complexity
	// also i dont wanna
	for i in 0..items.len()-1 {
		let mut mj = i;
		for j in i+1..items.len() {
			if items[j].date.unwrap() > items[mj].date.unwrap() {
				mj = j;
			}
		}
		items.swap(i, mj);
	}
}

fn update_item(amitems: Arc<Mutex<Vec<FeedItem>>>, currdate: u64, ii: usize) {
	let items = amitems.lock().unwrap();
	if items[ii].lookup.is_none() { return; }
	let f: String = items[ii].lookup.as_ref().unwrap().into();
	drop(items);
	println!("checking site {}", f);
	let currhash = get_hash_for_site(&f);
	let mut items = amitems.lock().unwrap();
	if currhash.is_some()
	&& (items[ii].hash.is_none()
	    || currhash.unwrap() != items[ii].hash.unwrap()) {
		if let Some(h) = items[ii].hash {
			println!("old hash: {}", h);
		}
		if let Some(h) = currhash {
			println!("old hash: {}", h);
		}
		items[ii].hash = Some(currhash.unwrap());
		items[ii].date = Some(currdate);
	}
	drop(items);
}

pub fn update_feed() {
	let items = read_feed();
	let currdate = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH).unwrap()
		.as_secs();
	let l = items.len();
	let amitems = Arc::new(Mutex::new(items));
	for i in 0..l {
		let h = amitems.clone();
		update_item(h, currdate, i);
	}
	let mut items = amitems.lock().unwrap();
	sort_feed(&mut items);
	save_feed_html(&items);
	save_feed(&items);
	println!("done updating feed");
}
