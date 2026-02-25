# Reality

An real time note collaboration web app, that saves your notes with a code. Share the code, and you have invite others to see and take notes with you. Reality's name is a play on the app this was supposed to mirror, notion.

http://reality.alokcampbell.space

## Features

- Make a document, write what you want, it is saved!
- Share the code you see next to Reality, and friends can join and write!
- Use markdown logic to make your text pop, and also allowing you to use the website to test it out!
- The document is always saved when you leave, remember the code and you can always come back!

## Setup

1. Clone the repository:
```bash
git clone https://github.com/alokcampbell/reality.git
cd reality
```

2. Then create the application, and build Dioxus:
```bash
dx build --release --platform web
cargo build --release --bin reality-server --features server
```

3. After that, run it:
```bash
./target/release/reality-server
```

There you go! Now you can connect to your website, and it should work!