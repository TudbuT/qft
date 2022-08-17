use std::{
    collections::HashMap,
    env,
    fs::{File, OpenOptions},
    io::{stdout, Error, Read, Seek, SeekFrom, Write},
    net::*,
    str::FromStr,
    thread,
    time::{Duration, SystemTime},
};

#[derive(Ord, Eq, PartialOrd, PartialEq)]
enum SafeReadWritePacket {
    Write,
    Ack,
    ResendRequest,
    End,
}

struct SafeReadWrite {
    socket: UdpSocket,
    last_transmitted: HashMap<u16, Vec<u8>>,
    packet_count_out: u64,
    packet_count_in: u64,
}

impl SafeReadWrite {
    pub fn new(socket: UdpSocket) -> SafeReadWrite {
        SafeReadWrite {
            socket,
            last_transmitted: HashMap::new(),
            packet_count_in: 0,
            packet_count_out: 0,
        }
    }

    pub fn write_safe(&mut self, buf: &[u8]) -> Result<(), Error> {
        if buf.len() > 0xfffc {
            panic!(
                "too large data packet sent over SafeReadWrite ({} > 0xfffc)",
                buf.len()
            );
        }

        let id = (self.packet_count_out as u16).to_be_bytes();
        let idn = self.packet_count_out as u16;
        self.packet_count_out += 1;

        let mut vbuf = Vec::from(buf);
        vbuf.insert(0, SafeReadWritePacket::Write as u8);
        vbuf.insert(0, id[1]);
        vbuf.insert(0, id[0]); // this is now the first byte
        let buf = vbuf.as_slice();

        loop {
            match self.socket.send(buf) {
                Ok(x) => {
                    if x != buf.len() {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            }
            self.last_transmitted.insert(u16::from_be_bytes(id), vbuf);
            break;
        }
        let mut buf = [0, 0, 0];
        if self.last_transmitted.len() < 50 {
            self.socket
                .set_read_timeout(Some(Duration::from_millis(1)))
                .unwrap();
        }
        let mut wait = idn == 0xffff;
        if wait {
            print!("\r\x1b[KPacket ID needs to wrap. Waiting for partner to catch up...")
        }
        let mut is_catching_up = false;
        loop {
            match self.socket.recv(&mut buf).ok() {
                Some(x) => {
                    if x != 3 {
                        continue;
                    }
                    if buf[2] == SafeReadWritePacket::Ack as u8 {
                        let n = u16::from_be_bytes([buf[0], buf[1]]);
                        self.last_transmitted.remove(&n);
                        if n == idn {
                            if idn == 0xffff {
                                println!("\r\x1b[KPacket ID wrap successful.");
                            }
                            wait = false;
                            self.last_transmitted.clear(); // if the latest packet is ACK'd, all
                                                           // previous ones must be as well.
                        }
                    }
                    if buf[2] == SafeReadWritePacket::ResendRequest as u8 {
                        let mut n = u16::from_be_bytes([buf[0], buf[1]]);
                        if !is_catching_up {
                            println!("\r\x1b[KA packet dropped: {}", &n);
                        }
                        wait = true;
                        is_catching_up = true;
                        while n <= idn && !(idn == 0xffff && n == 0) {
                            let buf = self.last_transmitted.get(&n).expect(
                                format!(
                                    "tried to ResendRequest an Ack'd packet with ID {}. Current ID: {}",
                                    &n, &idn
                                )
                                .as_str(),
                            );
                            loop {
                                // resend until success
                                match self.socket.send(&buf.as_slice()) {
                                    Ok(x) => {
                                        if x != buf.len() {
                                            continue;
                                        }
                                    }
                                    Err(_) => {
                                        continue;
                                    }
                                };
                                break;
                            }
                            // do NOT remove from last_transmitted yet, wait for Ack to do that.
                            n += 1;
                        }
                    }
                }
                None => {
                    if !wait {
                        break;
                    }
                }
            }
        }
        self.socket
            .set_read_timeout(Some(Duration::from_millis(1000)))
            .unwrap();
        return Ok(());
    }

    pub fn read_safe(&mut self, buf: &[u8]) -> Result<(Vec<u8>, usize), Error> {
        if buf.len() > 0xfffc {
            panic!(
                "attempted to receive too large data packet with SafeReadWrite ({} > 0xfffc)",
                buf.len()
            );
        }

        let mut mbuf = Vec::from(buf);
        mbuf.insert(0, 0);
        mbuf.insert(0, 0);
        mbuf.insert(0, 0);
        let buf: &mut [u8] = mbuf.as_mut();

        let mut r = (vec![], 0);

        let mut try_again = true;
        let mut is_catching_up = false;
        while try_again {
            match self.socket.recv(buf) {
                Ok(x) => {
                    if x < 3 {
                        continue;
                    }
                    let id = u16::from_be_bytes([buf[0], buf[1]]);
                    if id <= self.packet_count_in as u16 {
                        self.socket
                            .send(&[buf[0], buf[1], SafeReadWritePacket::Ack as u8])
                            .expect("send error");
                    }
                    if id == self.packet_count_in as u16 {
                        if id == 0xffff {
                            println!("\r\x1b[KPacket ID wrap successful.");
                        }
                        try_again = false;
                        self.packet_count_in += 1;
                        r.1 = x - 3;
                    } else if id > self.packet_count_in as u16 && (id - self.packet_count_in as u16) < 0xC000 {
                        if !is_catching_up {
                            println!(
                                "\r\x1b[KA packet dropped: {} (got) is newer than {} (expected)",
                                &id,
                                &(self.packet_count_in as u16)
                            );
                        }
                        is_catching_up = true;
                        // ask to resend, then do nothing
                        let id = (self.packet_count_in as u16).to_be_bytes();
                        self.socket
                            .send(&[id[0], id[1], SafeReadWritePacket::ResendRequest as u8])
                            .expect("send error");
                    }
                    if buf[2] == SafeReadWritePacket::End as u8 {
                        return Ok((vec![], 0));
                    }
                }
                Err(_) => {}
            }
        }
        mbuf.remove(0);
        mbuf.remove(0);
        mbuf.remove(0);
        r.0 = mbuf;
        return Ok(r);
    }

    pub fn end(mut self) -> UdpSocket {
        let id = (self.packet_count_out as u16).to_be_bytes();
        self.packet_count_out += 1;

        let mut vbuf = Vec::new();
        vbuf.insert(0, SafeReadWritePacket::End as u8);
        vbuf.insert(0, id[1]);
        vbuf.insert(0, id[0]); // this is now the first byte
        let buf = vbuf.as_slice();

        loop {
            match self.socket.send(buf) {
                Ok(x) => {
                    if x != buf.len() {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            }
            self.last_transmitted.insert(u16::from_be_bytes(id), vbuf);
            break;
        }
        let mut buf = [0, 0, 0];
        while self.last_transmitted.len() != 0 {
            match self.socket.recv(&mut buf).ok() {
                Some(x) => {
                    if x != 3 {
                        continue;
                    }
                    if buf[2] == SafeReadWritePacket::Ack as u8 {
                        self.last_transmitted
                            .remove(&u16::from_be_bytes([buf[0], buf[1]]));
                    }
                    if buf[2] == SafeReadWritePacket::ResendRequest as u8 {
                        let buf = self
                            .last_transmitted
                            .get(&u16::from_be_bytes([buf[0], buf[1]]))
                            .expect("tried to ResendRequest an Ack'd packet");
                        println!("\nCatching up...");
                        loop {
                            // resend until success
                            match self.socket.send(&buf.as_slice()) {
                                Ok(x) => {
                                    if x != buf.len() {
                                        continue;
                                    }
                                }
                                Err(_) => {
                                    continue;
                                }
                            };
                            break;
                        }
                        // do NOT remove from last_transmitted yet, wait for Ack to do that.
                    }
                }
                None => {}
            }
        }

        self.socket
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        print_args(&args);
    }
    match args
        .get(1)
        .unwrap() // checked in previous if-statement
        .as_str()
    {
        "helper" => helper(&args),
        "sender" => sender(&args),
        "receiver" => receiver(&args),
        "version" => println!("QFT version: {}", env!("CARGO_PKG_VERSION")),
        _ => print_args(&args),
    }
}

fn helper(args: &Vec<String>) {
    let bind_addr = (
        "0.0.0.0",
        u16::from_str_radix(args[2].as_str(), 10).expect("invalid port: must be integer"),
    );
    let mut map: HashMap<[u8; 200], SocketAddr> = HashMap::new();
    let listener = UdpSocket::bind(&bind_addr).expect("unable to create socket");
    let mut buf = [0 as u8; 200];
    loop {
        let (l, addr) = listener.recv_from(&mut buf).expect("read error");
        if l != 200 {
            continue;
        }
        if map.contains_key(&buf) {
            let other = map.get(&buf).unwrap();
            // we got a connection
            let mut bytes: &[u8] = addr.to_string().bytes().collect::<Vec<u8>>().leak();
            let mut addr_buf = [0 as u8; 200];
            for i in 0..bytes.len().min(200) {
                addr_buf[i] = bytes[i];
            }
            bytes = other.to_string().bytes().collect::<Vec<u8>>().leak();
            let mut other_buf = [0 as u8; 200];
            for i in 0..bytes.len().min(200) {
                other_buf[i] = bytes[i];
            }
            if listener.send_to(&addr_buf, other).is_ok()
                && listener.send_to(&other_buf, addr).is_ok()
            {
                // success!
                println!("Helped {} and {}! :D", addr, other);
            }
            map.remove(&buf);
        } else {
            map.insert(buf, addr);
        }
    }
}

fn sender(args: &Vec<String>) {
    let connection = holepunch(args);
    let br = u32::from_str_radix(args.get(5).unwrap_or(&"256".to_string()), 10)
        .expect("This is not a correct number");
    let begin = args
        .get(6)
        .map(|s| u64::from_str_radix(s.as_str(), 10))
        .unwrap_or(Ok(0))
        .expect("bad begin operand");
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(br as usize, 0);
    let mut buf = buf.leak();
    let mut file = File::open(args.get(4).unwrap_or_else(|| {
        print_args(args);
        panic!("unreachable")
    }))
    .expect("file not readable");

    if begin != 0 {
        println!("Skipping to {}...", begin);
        file.seek(SeekFrom::Start(begin)).expect("unable to skip");
        println!("Done.");
    }

    let mut sc = SafeReadWrite::new(connection);
    let mut bytes_sent: u64 = 0;
    loop {
        let read = file.read(&mut buf).expect("file read error");
        if read == 0 && !env::var("QFT_STREAM").is_ok() {
            println!();
            println!("Transfer done. Thank you!");
            sc.end();
            return;
        }

        sc.write_safe(&buf[..read]).expect("send error");
        bytes_sent += read as u64;
        if (bytes_sent % (br * 20) as u64) < (br as u64) {
            print!("\r\x1b[KSent {} bytes", bytes_sent);
            stdout().flush().unwrap();
        }
    }
}

fn receiver(args: &Vec<String>) {
    let connection = holepunch(args);
    let br = u32::from_str_radix(args.get(5).unwrap_or(&"256".to_string()), 10)
        .expect("This is not a correct number");
    let begin = args
        .get(6)
        .map(|s| u64::from_str_radix(s.as_str(), 10))
        .unwrap_or(Ok(0))
        .expect("bad begin operand");
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(br as usize, 0);
    let buf: &[u8] = buf.leak();
    let mut file = OpenOptions::new()
        .truncate(false)
        .write(true)
        .create(true)
        .open(&args.get(4).unwrap_or_else(|| {
            print_args(args);
            panic!("unreachable")
        }))
        .expect("file not writable");

    if begin != 0 {
        println!("Skipping to {}...", begin);
        file.seek(SeekFrom::Start(begin)).expect("unable to skip");
        println!("Done.");
    }

    let mut sc = SafeReadWrite::new(connection);
    let mut bytes_received: u64 = 0;
    loop {
        let (mbuf, len) = sc.read_safe(buf).expect("read error");
        let buf = &mbuf.leak()[..len];
        if len == 0 {
            println!();
            println!("Transfer done. Thank you!");
            return;
        }

        file.write(buf).expect("write error");
        file.flush().expect("file flush error");
        bytes_received += len as u64;
        if (bytes_received % (br * 20) as u64) < (br as u64) {
            print!("\r\x1b[KReceived {} bytes", bytes_received);
            stdout().flush().unwrap();
        }
    }
}

fn holepunch(args: &Vec<String>) -> UdpSocket {
    let bind_addr = (Ipv4Addr::from(0 as u32), 0);
    let holepunch = UdpSocket::bind(&bind_addr).expect("unable to create socket");
    holepunch
        .connect(args.get(2).unwrap_or_else(|| {
            print_args(args);
            panic!("unreachable")
        }))
        .expect("unable to connect to helper");
    let bytes = args
        .get(3)
        .unwrap_or_else(|| {
            print_args(args);
            panic!("unreachable")
        })
        .as_bytes();
    let mut buf = [0 as u8; 200];
    for i in 0..bytes.len().min(200) {
        buf[i] = bytes[i];
    }
    holepunch.send(&buf).expect("unable to talk to helper");
    holepunch
        .recv(&mut buf)
        .expect("unable to receive from helper");
    // buf should now contain our partner's address data.
    let mut s = Vec::from(buf);
    s.retain(|e| *e != 0);
    let bind_addr = String::from_utf8_lossy(s.as_slice()).to_string();
    println!(
        "Holepunching {} (partner) and :{} (you).",
        bind_addr,
        holepunch.local_addr().unwrap().port()
    );
    holepunch
        .connect(SocketAddrV4::from_str(bind_addr.as_str()).unwrap())
        .expect("connection failed");
    holepunch
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    holepunch
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    if env::var("QFT_USE_TIMED_HOLEPUNCH").is_ok() {
        println!("Waiting...");
        let mut stop = false;
        while !stop {
            thread::sleep(Duration::from_millis(500 - (unix_millis() % 500)));
            println!("CONNECT {}", unix_millis());
            let _ = holepunch.send(&[0]);
            let result = holepunch.recv(&mut [0, 0]);
            if result.is_ok() && result.unwrap() == 1 {
                holepunch.send(&[0, 0]).expect("connection failed");
                let result = holepunch.recv(&mut [0, 0]);
                if result.is_ok() && result.unwrap() == 2 {
                    stop = true;
                }
            }
        }
    } else {
        println!("Connecting...");
        thread::sleep(Duration::from_millis(500 - (unix_millis() % 500)));
        for _ in 0..40 {
            let m = unix_millis();
            let _ = holepunch.send(&[0]);
            thread::sleep(Duration::from_millis((50 - (unix_millis() - m)).max(0)));
        }
        let mut result = Ok(1);
        while result.is_ok() && result.unwrap() == 1 {
            result = holepunch.recv(&mut [0, 0]);
        }
        holepunch.send(&[0, 0]).expect("connection failed");
        holepunch.send(&[0, 0]).expect("connection failed");
        result = Ok(1);
        while result.is_ok() && result.unwrap() != 2 {
            result = holepunch.recv(&mut [0, 0]);
        }
        result = Ok(1);
        while result.is_ok() && result.unwrap() == 2 {
            result = holepunch.recv(&mut [0, 0]);
        }
    }
    println!("Holepunch and connection successful.");
    return holepunch;
}

fn print_args(args: &Vec<String>) {
    let f = args.get(0).unwrap();
    println!(
        "No arguments. Needed: \n\
         | {} helper <bind-port>\n\
         | {} sender <helper-address>:<helper-port> <phrase> <filename> [bitrate] [skip]\n\
         | {} receiver <helper-address>:<helper-port> <phrase> <filename> [bitrate] [skip]\n\
         | {} version\n",
        f, f, f, f
    );
    panic!("No arguments");
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
