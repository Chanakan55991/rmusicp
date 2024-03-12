#![forbid(unsafe_code)]

use regex::Regex;
use rodio::Sink;
use rodio::{Decoder, OutputStream};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::exit;
use tempfile::TempDir;
use ytd_rs::{Arg, YoutubeDL};

fn main() {
    let mut should_exit = false;
    let tmp_dir = TempDir::new().unwrap_or_else(|err| {
        eprintln!("Error Occurred while creating temporary directory: {err}");
        exit(1)
    });

    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let youtube_regex = Regex::new(
        r"(http:|https:)?(\/\/)?(www\.)?(youtube\.com|youtu\.be)/(watch\?v=)?([a-zA-Z0-9_-]{11})",
    )
    .unwrap();
    let yt_args = vec![
        Arg::new_with_arg("-f", "bestaudio"),
        Arg::new("-x"),
        Arg::new_with_arg("--audio-format", "flac"),
    ];

    let (_stream, stream_handle) = OutputStream::try_default().unwrap_or_else(|err| {
        eprintln!("Error Occurred while creating audio stream, do you have an audio output devices?: {err}");
        exit(1)
    });

    let sink = Sink::try_new(&stream_handle).unwrap_or_else(|err| {
        eprintln!("Error Occurred while creating audio sink: {err}");
        exit(1)
    });

    while !should_exit {
        print!("> ");
        Write::flush(&mut std::io::stdout()).expect("flush failed!");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        let mut input = input.trim().split(" ");
        let command = input.next();

        match command.unwrap() {
            "play" => {
                let path = input.next().unwrap_or("");
                let mut file = File::open(path); // simply set it to handle local file first
                if youtube_regex.is_match(path) {
                    // then if found out it's a youtube link, handle differently
                    println!("YouTube Link Found, using yt-dlp to download audio...");
                    let dir = PathBuf::from(tmp_dir.path());
                    let ytd = YoutubeDL::new(&dir, yt_args.clone(), &path);

                    if ytd.is_err() {
                        eprintln!(
                            "Error occurred while creating yt-dlp instance: {}",
                            ytd.err().unwrap()
                        );
                        continue;
                    }

                    let download = ytd.unwrap().download();
                    if download.is_err() {
                        eprintln!(
                            "Error occurred while downloading audio: {}",
                            download.err().unwrap()
                        );
                        continue;
                    }

                    let download = download.unwrap();
                    let filepath = download.output_dir().to_string_lossy().to_string();
                    let output = download.output().split("\n");

                    for line in output {
                        if line.contains("[ExtractAudio] Destination:") {
                            let path = line
                                .split("[ExtractAudio] Destination: ")
                                .collect::<Vec<&str>>()[1];
                            let fullpath = format!("{}/{}", filepath, path);
                            file = File::open(fullpath);
                            break;
                        }
                    }
                }

                if file.is_err() {
                    eprintln!("File not found");
                    continue;
                }
                let buf = BufReader::new(file.unwrap()); // should be safe to unwrap since we check for errors beforehand
                let source = Decoder::new(buf);
                if source.is_err() {
                    eprintln!("Error occurred while decoding audio file");
                    continue;
                }

                if sink.len() == 0 {
                    sink.play();
                    println!("Playing Audio...");
                } else {
                    println!("Adding audio to queue...");
                }
                sink.append(source.unwrap());
            }
            "pause" => {
                sink.pause();
            }
            "resume" => {
                sink.play();
            }
            "p" => {
                if sink.is_paused() {
                    sink.play();
                } else {
                    sink.pause();
                }
            }
            "n" | "next" => {
                if sink.len() == 0 {
                    println!("No more audio to play");
                } else {
                    println!("Playing Next Song...");
                }
                sink.skip_one();
            }
            "s" | "skip" => {
                let skip_amount = input.next().unwrap_or("1").parse::<usize>().unwrap_or(1);
                println!("Skipping {skip_amount} Song(s)...");

                for _ in 0..skip_amount {
                    if sink.len() == 0 {
                        println!("No more audio to play");
                        break;
                    }
                    sink.skip_one();
                }
            }
            "clear" | "c" | "stop" => {
                sink.clear();
                println!("Audio Stopped");
            }
            "exit" => {
                should_exit = true;
            }
            "" => {}
            _ => println!("Unknown command"),
        }
        if command.unwrap() != "" {
            if sink.len() > 0 {
                println!("Position in queue: {length}", length = sink.len());
            } else {
                println!("No audio in queue");
            }
        }
    }

    if should_exit {
        println!("Exiting...");
        sink.stop();
        let _ = tmp_dir.close();
        exit(0);
    }
}
