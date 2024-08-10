use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use std::net::SocketAddr;
use std::env;
use std::convert::Infallible;
use std::path::Path;
use hyper::header::{CONTENT_DISPOSITION, HeaderValue};
use qrcodegen::{QrCode, QrCodeEcc};
use std::process::Command;
use rand::Rng;
use image::Luma;

async fn handle_request(req: Request<Body>, file_path: String) -> Result<Response<Body>, Infallible> {
    if req.uri().path() == "/download" {
        let mut file = File::open(&file_path).await.unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await.unwrap();

        let file_name = Path::new(&file_path).file_name().unwrap().to_str().unwrap();
        let mut response = Response::new(Body::from(contents));

        // Set the Content-Disposition header to suggest a file download with the correct file name
        let content_disposition = format!("attachment; filename=\"{}\"", file_name);
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&content_disposition).unwrap(),
        );

        Ok(response)
    } else {
        Ok(Response::new(Body::from("File not found.")))
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        return;
    }

    let file_path = args[1].clone();

    // Bind to a local address with a random port
    let port = rand::thread_rng().gen_range(1024..65535); // Random port between 1024 and 65535
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let make_svc = make_service_fn(move |_| {
        let file_path = file_path.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, file_path.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    // Get the local IP address and generate the download link
    let local_ip = get_local_ip().unwrap_or_else(|e| {
        eprintln!("Failed to get local IP address: {}", e);
        "127.0.0.1".to_string()
    });
    let download_link = format!("http://{}:{}/download", local_ip, addr.port());

    // Print the download link
    println!("Download link: {}", download_link);

    // Generate and save the QR code using the download link
    let qr_code_path = "qr_code.png";
    let code = QrCode::encode_text(&download_link, QrCodeEcc::Medium).unwrap();
    save_qr_code_as_png(&code, qr_code_path).unwrap();

    // Display the QR code image (not the link) in the terminal using qrencode
    let qrencode_process = Command::new("qrencode")
        .arg("-t")
        .arg("ansiutf8")
        .arg("-o")
        .arg("-") // Output to stdout
        .arg(&download_link) // Generate QR code from the download link
        .status()
        .expect("Failed to generate QR code output");

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn get_local_ip() -> Result<String, std::io::Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ifconfig | grep inet | awk '{print $2}' | grep -v '127.0.0.1' | head -n1")
        .output()?;

    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ip.is_empty() {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No IP address found"))
    } else {
        Ok(ip)
    }
}

fn save_qr_code_as_png(code: &QrCode, file_path: &str) -> Result<(), image::ImageError> {
    let size = code.size();
    let scale = 10; // Scale factor to enlarge the QR code
    let border = 4;
    let img_size = (size + border * 2) * scale;
    let mut img = image::ImageBuffer::new(img_size as u32, img_size as u32);

    for y in 0..img_size {
        for x in 0..img_size {
            let module_x = (x / scale) as i32 - border;
            let module_y = (y / scale) as i32 - border;
            let color = if module_x >= 0 && module_x < size && module_y >= 0 && module_y < size && code.get_module(module_x, module_y) {
                Luma([0u8]) // Black
            } else {
                Luma([255u8]) // White
            };
            img.put_pixel(x as u32, y as u32, color);
        }
    }

    img.save(file_path)
}

