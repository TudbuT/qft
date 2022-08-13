use std::{
    collections::HashMap,
    fs::File,
    io::{Error, Read, Write},
    net::*,
    str::FromStr,
    thread,
    time::{Duration, SystemTime},
};

trait Ordinal {
    fn ordinal(&self) -> isize;
}

enum SafeReadWritePacket {
    WRITE,
    ACK,
    END,
}

impl Ordinal for SafeReadWritePacket {
    fn ordinal(&self) -> isize {
        match self {
            SafeReadWritePacket::WRITE => 0,
            SafeReadWritePacket::ACK => 1,
            SafeReadWritePacket::END => 2,
        }
    }
}

struct SafeReadWrite {
    socket: UdpSocket,
    packet_count_out: u64,
    packet_count_in: u64,
}

impl SafeReadWrite {
    pub fn new(socket: UdpSocket) -> SafeReadWrite {
        SafeReadWrite {
            socket,
            packet_count_in: 0,
            packet_count_out: 0,
        }
    }

    pub fn write_safe(&mut self, buf: &[u8]) -> Result<(), Error> {
        if buf.len() > 0xfffd {
            panic!(
                "too large data packet sent over SafeReadWrite ({} > 0xfffd)",
                buf.len()
            );
        }

        let id = self.packet_count_out as u8;
        self.packet_count_out += 1;

        let mut buf = Vec::from(buf);
        buf.insert(0, SafeReadWritePacket::WRITE.ordinal() as u8);
        buf.insert(0, id);
        let buf = buf.as_slice();

        self.socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("cannot set_read_timeout");

        let mut resend = true;
        while resend {
            match self.socket.send(buf) {
                Ok(x) => {
                    if x != buf.len() {
                        panic!("bad buf length")
                    }
                }
                Err(x) => return Err(x),
            }
            let mut buf = [0, 0];
            match self.socket.recv(&mut buf).ok() {
                Some(x) => {
                    if x == 0 {
                        continue;
                    }
                    if buf[1] == SafeReadWritePacket::ACK.ordinal() as u8 && buf[0] == id {
                        resend = false;
                    }
                }
                None => {}
            }
        }
        return Ok(());
    }

    pub fn read_safe(&mut self, buf: &[u8]) -> Result<(Vec<u8>, usize), Error> {
        if buf.len() > 0xfffd {
            panic!(
                "attempted to receive too large data packet with SafeReadWrite ({} > 0xfffd)",
                buf.len()
            );
        }

        let mut mbuf = Vec::from(buf);
        mbuf.insert(0, 0);
        mbuf.insert(0, 0);
        let buf: &mut [u8] = mbuf.as_mut();

        let mut r = (vec![], 0);

        let mut try_again = true;
        while try_again {
            match self.socket.recv(buf) {
                Ok(x) => {
                    if x == 0 {
                        continue;
                    }
                    if buf[0] <= self.packet_count_in as u8 {
                        self.socket
                            .send(&[buf[0], SafeReadWritePacket::ACK.ordinal() as u8])
                            .expect("send error");
                    }
                    if buf[0] == self.packet_count_in as u8 {
                        try_again = false;
                        self.packet_count_in += 1;
                        r.1 = x - 2;
                    }
                    if buf[1] == SafeReadWritePacket::END.ordinal() as u8 {
                        return Ok((vec![], 0));
                    }
                }
                Err(x) => return Err(x),
            }
        }
        mbuf.remove(0);
        mbuf.remove(0);
        r.0 = mbuf;
        return Ok(r);
    }

    pub fn end(mut self) -> UdpSocket {
        let id = self.packet_count_out as u8;
        self.packet_count_out += 1;

        let mut buf = vec![];
        buf.insert(0, SafeReadWritePacket::END.ordinal() as u8);
        buf.insert(0, id);
        let buf = buf.as_slice();

        self.socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("cannot set_read_timeout");

        let mut resend = true;
        while resend {
            match self.socket.send(buf) {
                Ok(x) => {
                    if x != buf.len() {
                        panic!("internet down")
                    }
                }
                Err(_) => return self.socket,
            }
            let mut buf = [0, 0];
            match self.socket.recv(&mut buf).ok() {
                Some(x) => {
                    if x == 0 {
                        continue;
                    }
                    if buf[1] == SafeReadWritePacket::ACK.ordinal() as u8 && buf[0] == id {
                        resend = false;
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
        .expect("no args supplied, check if the first arg is truly the program name")
        .as_str()
    {
        "helper" => helper(&args),
        "sender" => sender(&args),
        "receiver" => receiver(&args),
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
    let br = u32::from_str_radix(args.get(5).unwrap_or(&"256".to_string()), 10).expect("This is not a correct number");
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(br as usize, 0);
    let mut buf = buf.leak();
    let mut file = File::open(args.get(4).unwrap_or_else(|| {
        print_args(args);
        panic!("unreachable")
    }))
    .expect("file not readable");

    let mut sc = SafeReadWrite::new(connection);
    let mut bytes_sent: u64 = 0;
    loop {
        let read = file.read(&mut buf).expect("file read error");
        if read == 0 {
            println!();
            println!("Transfer done. Thank you!");
            sc.end();
            return;
        }

        sc.write_safe(&buf[..read]).expect("send error");
        bytes_sent += read as u64;
        print!("\rSent {} bytes", bytes_sent);
    }
}

fn receiver(args: &Vec<String>) {
    let connection = holepunch(args);
    let br = u32::from_str_radix(args.get(5).unwrap_or(&"256".to_string()), 10).expect("This is not a correct number");
    let mut buf: Vec<u8> = Vec::new();
    buf.resize(br as usize, 0);
    let mut buf: &[u8] = buf.leak();
    let mut file = File::create(args.get(4).unwrap_or_else(|| {
        print_args(args);
        panic!("unreachable")
    }))
    .expect("file not writable");

    let mut sc = SafeReadWrite::new(connection);
    let mut bytes_received: u64 = 0;
    loop {
        let (mbuf, len) = sc.read_safe(buf).expect("read error");
        buf = &mbuf.leak()[..len];
        if len == 0 {
            println!();
            println!("Transfer done. Thank you!");
            return;
        }

        file.write(buf).expect("write error");
        bytes_received += len as u64;
        print!("\rReceived {} bytes", bytes_received);
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
    println!("Waiting...");
    let mut stop = false;
    while !stop {
        let m = unix_millis();
        thread::sleep(Duration::from_millis(500 - (m % 500)));
        println!("CONNECT {}", unix_millis());
        holepunch.send(&[0]).expect("connection failed");
        if holepunch.recv(&mut [0]).is_ok() {
            stop = true;
        }
    }
    println!(
        "Holepunch and connection successful. Running with {} (partner) and :{} (you).",
        bind_addr,
        holepunch.local_addr().unwrap().port()
    );
    return holepunch;
}

fn print_args(args: &Vec<String>) {
    let f = args.get(0).unwrap();
    println!(
        "No arguments. Needed: \n\
         | {} helper <bind-port>\n\
         | {} sender <helper-address>:<helper-port> <phrase> <filename> [bitrate]\n\
         | {} receiver <helper-address>:<helper-port> <phrase> <filename> [bitrate]",
        f, f, f
    );
    panic!("No arguments");
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
