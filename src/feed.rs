use std::io::{Read, Write};
use std::fs::File;
use std::time::SystemTime;
use std::sync::{Arc, Mutex};
use crate::wirt;

fn beginstrb(h: &[u8], n: &[u8]) -> bool {
	if n.len() > h.len() { return false; }
	for i in 0..n.len() {
		if h[i] != n[i] { return false; }
	}
	return true
}

async fn get_hash_for_site(url: &str) -> Option<u32> {
	let boi = reqwest::get(url).await;
	match boi {
	Err(_) => {
		return None;
	},
	_ => {}
	}
	let boi = boi.unwrap().bytes().await.unwrap();
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
	wirt::html_template(&mut file, "a.html");
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
		file.write(b"<br/>").unwrap();
		if let Some(h) = item.date {
			file.write(
				format!("{} ", currdate - h).as_bytes()
			).unwrap();
		}
		file.write(
			format!("<a href=\"{}\">{}</a>", url, text).as_bytes()
		).unwrap();
		file.write(b"\n").unwrap();
	}
	wirt::html_template(&mut file, "z.html");
}

fn sort_feed(items: &mut Vec<FeedItem>) {
	// selection sort
	// also not worried about complexity
	// also i dont wanna
	for i in 0..items.len()-1 {
		let mut mj = i;
		for j in i+1..items.len() {
			if items[j].date.is_none() {}
			else if
			items[mj].date.is_none() ||
			items[j].date.unwrap() > items[mj].date.unwrap()
			{ mj = j; }
		}
		items.swap(i, mj);
	}
}

async fn update_item(amitems: Arc<Mutex<Vec<FeedItem>>>, currdate: u64, ii: usize) {
	let items = amitems.lock().unwrap();
	if items[ii].lookup.is_none() { println!("kdjafl"); return; }
	let f: String = items[ii].lookup.as_ref().unwrap().into();
	drop(items);
	let currhash = get_hash_for_site(&f).await;
	let mut items = amitems.lock().unwrap();
	if currhash.is_some() && items[ii].hash.is_some() {
		// visited page before and it is different now
		if currhash.unwrap() != items[ii].hash.unwrap() {
			println!("site {}", f);
			println!("old hash: {}", items[ii].hash.unwrap());
			println!("new hash: {}", currhash.unwrap());
			items[ii].hash = Some(currhash.unwrap());
			items[ii].date = Some(currdate);
		}
	} else if items[ii].date.is_none() && items[ii].hash.is_none() {
		// visiting page first time, update hash and not last update tm
		if let Some(h) = currhash {
			println!("site {}", f);
			println!("new hash: {}", currhash.unwrap());
			items[ii].hash = Some(h);
		}
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
	let rt = tokio::runtime::Runtime::new().unwrap();
	let am = amitems.clone();
	rt.block_on(async move {
		let mut futs = Vec::new();
		for i in 0..l {
			let h = am.clone();
			futs.push(update_item(h, currdate, i));
		}
		futures::future::join_all(futs).await;
	});
	let mut items = amitems.lock().unwrap();
	println!("sorting feed");
	sort_feed(&mut items);
	println!("saving feed");
	save_feed_html(&items);
	save_feed(&items);
	println!("done updating feed");
}
