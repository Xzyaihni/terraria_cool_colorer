use std::env;

use std::process;

use std::thread;

use std::io::{Write, BufReader, BufRead, ErrorKind};
use std::net::{TcpStream, TcpListener};

use colorer::{Colorer, Color, Interpolation};

mod colorer;


struct ColorParser
{
    colors: Vec<String>
}

impl ColorParser
{
    pub fn new(colors: String) -> Self
    {
        let colors = colors.split([',', ';']).map(|s| s.to_string()).collect();
        ColorParser{colors}
    }

    pub fn parse(&self) -> Result<Vec<Color>, String>
    {
        let mut colors = self.colors.iter();

        let mut parsed = Vec::new();
        while let Some(r) = colors.next()
        {
            let r = r.as_str();
            let g = colors.next().ok_or("no green value".to_string())?;
            let b = colors.next().ok_or("no blue value".to_string())?;

            parsed.push(Color::try_from([r, g, b])?);
        }

        Ok(parsed)
    }
}

struct Config
{
    connect_address: String,
    colors: Vec<Color>,
    shift: bool,
    interpolation: Interpolation,
    port: u32
}

impl Config
{
    pub fn parse(args: impl Iterator<Item=String>) -> Result<Self, String>
    {
        let mut connect_address = String::new();

        let mut colors = vec![
            Color::new(255, 0, 0),
            Color::new(0, 255, 0),
            Color::new(0, 0, 255)
            ];

        let mut shift = true;
        let mut interpolation = Interpolation::Linear;
        let mut port = 8888;

        let mut args = args.skip(1);
        while let Some(arg) = args.next()
        {
            match arg.as_str()
            {
                "-c" | "--connect-address" =>
                {
                    connect_address = args.next().ok_or(format!("{arg} has no argument"))?;
                },
                "-C" | "--colors" =>
                {
                    let colors_list = args.next().ok_or(format!("{arg} has no argument"))?;

                    colors = ColorParser::new(colors_list).parse()?;
                    if colors.len()==0
                    {
                        return Err(format!("{arg} has no colors"));
                    }
                },
                "-s" | "--shift" =>
                {
                    shift = false;
                },
                "-i" | "--interpolation" =>
                {
                    let interp_type = args.next().ok_or(format!("{arg} has no argument"))?;
                    interpolation = match interp_type.to_lowercase().as_str()
                    {
                        "random" => Ok(Interpolation::Random),
                        "nearest" => Ok(Interpolation::Nearest),
                        "linear" => Ok(Interpolation::Linear),
                        "cubic" => Ok(Interpolation::Cubic),
                        _ => Err(format!("{arg} is not a valid interpolation"))
                    }?;
                },
                "-p" | "--port" =>
                {
                    port = args.next().ok_or(format!("{arg} has no argument"))?
                        .parse().map_err(|err| format!("{err} cannot be converted to port"))?;
                },
                opt =>
                {
                    return Err(format!("unknown option: {opt}"));
                }
            }
        }

        if connect_address.is_empty()
        {
            return Err("must have -c or --connect-address option specified".to_string());
        }

        Ok(Config{connect_address, colors, shift, interpolation, port})
    }
}

fn help_message() -> !
{
    let executable = env::args().nth(0).unwrap();
    eprintln!("usage: {executable} [args]");
    eprintln!(" args:");
    eprintln!("    -c, --connect-address    address to connect to");
    eprintln!("    -C, --colors             gradient to use (default 255, 0, 0; 0, 255, 0; 0, 0, 255)");
    eprintln!("    -s, --shift              dont shift the colors randomly");
    eprintln!("    -i, --interpolation      interpolation type (see below, default linear)");
    eprintln!("    -p, --port               proxy port (default 8888)");
    eprintln!(" gradients:");
    eprintln!("    gradients are lists of 3 values (rgb) separated by , or ;");
    eprintln!("    example:");
    eprintln!("     shifts from red (255,0,0) to blue (0,0,255)");
    eprintln!("     255, 0, 0; 0, 0, 255");
    eprintln!(" interpolations:");
    eprintln!("    available interpolation types are: random, nearest, linear, cubic");
    process::exit(1);
}

fn main()
{
    let config = Config::parse(env::args()).unwrap_or_else(|err|
        {
            eprintln!("error: {err}\n");
            help_message();
        });

    start_listening(&config).unwrap_or_else(|err|
    {
        eprintln!("error: {err}");
    });
}

fn start_listening(config: &Config) -> Result<(), String>
{
    let listen_address = format!("127.0.0.1:{}", config.port);

    let listener = TcpListener::bind(&listen_address)
        .map_err(|err| format!("could not start a local server on {listen_address}: {err}"))?;

    println!("listening for incoming connections: {listen_address}");

    for stream in listener.incoming()
    {
        let mut write_stream = stream.map_err(|err| format!("could not establish connection: {err}"))?;

        let mut write_connector = TcpStream::connect(&config.connect_address)
            .map_err(|err| format!("could not connect to {}: {err}", &config.connect_address))?;

        let mut read_stream = write_stream.try_clone()
            .map_err(|err| format!("error cloning client stream: {err}"))?;
        let mut read_connector = write_connector.try_clone()
            .map_err(|err| format!("error cloning server stream: {err}"))?;

        let colorer =
            Colorer::new(
                config.colors.clone(),
                config.shift,
                config.interpolation.clone()
                );

        thread::spawn(move ||
        {
            ClientReader::spawn(&mut read_stream, &mut write_connector, colorer)
                .listen_connection();
        });

        thread::spawn(move ||
        {
            ServerReader::spawn(&mut read_connector, &mut write_stream)
                .listen_connection();
        });
    }

    Ok(())
}


trait StreamReader
{
    fn read_stream(&mut self) -> &mut TcpStream;

    fn handle_stream(
        &mut self,
        ) -> Result<Vec<u8>, String>
    {
        let mut reader = BufReader::new(self.read_stream());

        let buffer: Vec<u8> = reader.fill_buf()
            .map_err(|err| format!("error reading stream: {err}"))?.to_vec();
        Ok(self.handle_buffer(&buffer))
    }

    fn handle_buffer(&mut self, buffer: &[u8]) -> Vec<u8>;
}

trait ProxyPart<'a>: StreamReader
{
    fn write_stream(&mut self) -> &mut TcpStream;

    fn listen_connection(&mut self)
    {
        loop
        {
            match self.handle_stream()
            {
                Ok(data) =>
                {
                    match self.write_stream().write(&data)
                    {
                        Err(err) =>
                        {
                            if err.kind()==ErrorKind::BrokenPipe
                            {
                                println!("connection closed");
                                process::exit(0);
                            }
                            println!("error writing to out: {err}");
                            process::exit(1);
                        },
                        _ => ()
                    }
                    self.write_stream().flush().unwrap();
                },
                Err(err) => println!("error reading in data: {err}")
            }
        }
    }
}


struct ClientReader<'a>
{
    read_stream: &'a mut TcpStream,
    write_stream: &'a mut TcpStream,
    colorer: Colorer
}

impl<'a> ClientReader<'a>
{
    pub fn spawn(
        read_stream: &'a mut TcpStream,
        write_stream: &'a mut TcpStream,
        colorer: Colorer
        ) -> Self
    {
        ClientReader{
            read_stream,
            write_stream,
            colorer
            }
    }

    const MINIMUM_SIZE: usize = 10;

    const CHAT_MESSAGE_HEADER: [u8; 7] =
        [0x52, 0x01, 0x00, 0x03, 0x53, 0x61, 0x79];

    const MESSAGE_POS: usize = 9;

    fn change_chat(&mut self, buffer: &[u8]) -> Vec<u8>
    {
        let full_length = buffer.len()-Self::MESSAGE_POS;
        let length_length = if full_length>128
        {
            2
        } else
        {
            1
        };

        let real_msg_pos = Self::MESSAGE_POS+length_length;

        let message = String::from_utf8_lossy(&buffer[real_msg_pos..]);
        println!("client sent: {}", message);

        let new_message = self.colorer.color_text(&message);

        let new_length = new_message.bytes().len();
        let mut encoded_length = Self::terraria_type(new_length as u32);

        let mut out_vec = Vec::new();

        //length of the payload
        let payload_length = (Self::MESSAGE_POS+encoded_length.len()+new_length) as u16;
        out_vec.extend(payload_length.to_le_bytes().into_iter());

        //the header
        out_vec.extend(&Self::CHAT_MESSAGE_HEADER);

        //length ("""encoded""" in the dumbest way, why????)
        out_vec.append(&mut encoded_length);

        //message
        out_vec.extend(new_message.bytes());

        out_vec
    }

    fn terraria_type(value: u32) -> Vec<u8>
    {
        let length_mod = value%128;
        let mut full_msg = vec![length_mod as u8];
        if value>127
        {
            full_msg[0] += 128;
            let divisions = value/128_u32;

            let mult = divisions as u8;

            full_msg.push(mult);
        }
        full_msg
    }
}

impl<'a> StreamReader for ClientReader<'a>
{
    fn read_stream(&mut self) -> &mut TcpStream
    {
        self.read_stream
    }

    fn handle_buffer(&mut self, buffer: &[u8]) -> Vec<u8>
    {
        let size = buffer.len();
        if size>=Self::MINIMUM_SIZE && buffer[2..9]==Self::CHAT_MESSAGE_HEADER
        {
            self.change_chat(&buffer)
        } else
        {
            buffer.to_vec()
        }
    }
}

impl<'a> ProxyPart<'a> for ClientReader<'a>
{
    fn write_stream(&mut self) -> &mut TcpStream
    {
        self.write_stream
    }
}


struct ServerReader<'a>
{
    read_stream: &'a mut TcpStream,
    write_stream:  &'a mut TcpStream
}

impl<'a> ServerReader<'a>
{
    pub fn spawn(read_stream: &'a mut TcpStream, write_stream: &'a mut TcpStream) -> Self
    {
        ServerReader{
            read_stream,
            write_stream
            }
    }
}

impl<'a> StreamReader for ServerReader<'a>
{
    fn read_stream(&mut self) -> &mut TcpStream
    {
        self.read_stream
    }

    fn handle_buffer(&mut self, buffer: &[u8]) -> Vec<u8>
    {
        buffer.to_vec()
    }
}

impl<'a> ProxyPart<'a> for ServerReader<'a>
{
    fn write_stream(&mut self) -> &mut TcpStream
    {
        self.write_stream
    }
}