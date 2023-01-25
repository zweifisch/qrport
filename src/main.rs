use std::path::Path;

use qrcode_generator::{QrCodeEcc, QRCodeError};
use clap::Parser;
use qrport::http;
use local_ip_address::local_ip;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    path: Option<String>,

    #[arg(short, long, default_value_t = 8008)]
    port: u16,
}

fn print_qr_code(msg: &str) -> Result<(), QRCodeError> {

    let matrix = qrcode_generator::to_matrix(msg, QrCodeEcc::Low)?;

    for row in matrix {
        for cell in row {
            if cell {
                print!("\u{2588}\u{2588}");
            } else {
                print!("  ");
            }
        }
        println!("")
    }
    Ok(())
}

fn main() {

    let cli = Cli::parse();

    let ip = local_ip().unwrap();

    if let Some(path) = cli.path {
        let pth = Path::new(&path);
        print_qr_code(&format!(
            "http://{}:{}/{}", ip, cli.port, pth.file_name().unwrap().to_str().unwrap())).unwrap();
        http::serve(cli.port, move |req| {
            req.send_file(&path).unwrap();
            println!("{}", req.addr().unwrap().ip().to_string());
        });
    }
}
