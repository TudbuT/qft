use std::{
    fs::File,
    io::{Error, Read, Write},
    net::*,
    time::Duration,
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
    packet_count_out: u8,
    packet_count_in: u8,
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

        let id = self.packet_count_out;
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
                    if buf[0] <= self.packet_count_in {
                        self.socket
                            .send(&[buf[0], SafeReadWritePacket::ACK.ordinal() as u8])
                            .expect("send error");
                    }
                    if buf[0] == self.packet_count_in {
                        try_again = false;
                        self.packet_count_in += 1;
                        r.1 = x - 2;
                    }
                    if buf[0] > self.packet_count_in {
                        panic!("illegal packet id {} > {}", buf[0], self.packet_count_in);
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
        let id = self.packet_count_out;
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

    pub fn get_socket(&self) -> &UdpSocket {
        &self.socket
    }

    pub fn get_mut_socket(&mut self) -> &mut UdpSocket {
        &mut self.socket
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
        "127.0.0.1",
        u16::from_str_radix(args[2].as_str(), 10).expect("invalid port: must be integer"),
    );
    let listener = UdpSocket::bind(&bind_addr).expect("unable to create socket");
    let mut buf = [0 as u8];
    loop {
        let (_, addr) = listener.recv_from(&mut buf).expect("read error");
        // we got a connection
        if listener.send_to(&addr.port().to_be_bytes(), addr).is_ok() {
            // success!
            println!("Helped {}! :D", addr);
        }
    }
}

fn sender(args: &Vec<String>) {
    let mut bind_addr = ("127.0.0.1", 0);
    {
        let holepunch = UdpSocket::bind(&bind_addr).expect("unable to create socket");
        let mut buf = [0 as u8; 2];
        holepunch
            .connect(args.get(2).unwrap_or_else(|| {
                print_args(args);
                panic!("unreachable")
            }))
            .expect("unable to connect to helper");
        holepunch.send(&[0]).expect("unable to talk to helper");
        holepunch
            .recv(&mut buf)
            .expect("unable to receive from helper");
        // buf should now contain our port.
        bind_addr = ("127.0.0.1", u16::from_be_bytes(buf));
        println!("Holepunch successful. Running on port {}.", bind_addr.1);
    }
    // we have the needed bind_addr and did the holepunch
    {
        let connection = UdpSocket::bind(&bind_addr).expect("unable to create send socket");
        let mut buf = [0 as u8; 256];
        let mut file = File::open(args.get(3).unwrap_or_else(|| {
            print_args(args);
            panic!("unreachable")
        }))
        .expect("file not readable");
        connection
            .connect(connection.recv_from(&mut buf).expect("connect error").1)
            .expect("connect error");

        let mut sc = SafeReadWrite::new(connection);
        loop {
            let read = file.read(&mut buf).expect("file read error");
            if read == 0 {
                println!("Transfer done. Thank you!");
                sc.end();
                return;
            }

            sc.write_safe(&buf[..read]).expect("send error");
            println!("Sent {} bytes", read);
        }
    }
}

fn receiver(args: &Vec<String>) {
    let connection = UdpSocket::bind(("127.0.0.1", 0)).expect("unable to create receive socket");
    let mut buf: &[u8] = &[0 as u8; 256];
    let mut file = File::create(args.get(3).unwrap_or_else(|| {
        print_args(args);
        panic!("unreachable")
    }))
    .expect("file not writable");
    connection
        .connect(args.get(2).unwrap_or_else(|| {
            print_args(args);
            panic!("unreachable")
        }))
        .expect("unable to connect");
    connection.send(&[0]).expect("connect write error");

    let mut sc = SafeReadWrite::new(connection);
    loop {
        let (mbuf, len) = sc.read_safe(buf).expect("read error");
        buf = &mbuf.leak()[..len];
        if len == 0 {
            println!("Transfer done. Thank you!");
            return;
        }

        file.write(buf).expect("write error");
        println!("Received {} bytes", len);
    }
}

fn print_args(args: &Vec<String>) {
    let f = args.get(0).unwrap();
    println!(
        "No arguments. Needed: \n\
         | {} helper <bind-port>\n\
         | {} sender <helper-address>:<helper-port> <filename>\n\
         | {} receiver <sender-address>:<sender-port> <filename>",
        f, f, f
    );
    panic!("No arguments");
}
