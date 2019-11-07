use std::io::{Read, Write};

use termion::raw::IntoRawMode;

struct ResetTerm;

impl Drop for ResetTerm {
    fn drop(&mut self) {
        print!("\x1b[?1002l\x1b[?25h\x1b[?1049l");
    }
}

fn main() {
    let mut stdin = std::io::stdin();

    let mut stdout = std::io::stdout().into_raw_mode().unwrap();
    
    // do it as drop impl so that panic unwinding would reset the term too
    let _reset_term = ResetTerm;

    write!(stdout, "\x1b[?1049h\x1b[?25l\x1b[?1002h").unwrap();
    stdout.flush().unwrap();

    let mut state = 0;

    loop {
        let mut buf = [0; 1];
        stdin.read_exact(&mut buf).unwrap();
        let byte = buf[0];

        match byte {
            3 | 113 => break, // Ctrl+C or Q to exit
            12 => {           // Ctrl+L to clear the screen
                write!(stdout, "\x1b[s\x1b[2J\x1b[3J\x1b[u").unwrap();
                stdout.flush().unwrap();
                state = 0;
            }
            27 if state == 0 => state = 1, // \e
            91 if state == 1 => state = 2, // [
            77 if state == 2 => {          // M
                let mut buf = [0; 3];
                stdin.read_exact(&mut buf).unwrap();
                let b = buf[0] - 32;
                let x = buf[1] - 32;
                let y = buf[2] - 32;

//                write!(stdout, "{} ", b);
//                stdout.flush().unwrap();

                if b == 0 || b == 32 {
                    let p = "██";

                    write!(stdout, "\x1b[s\x1b[{1};{0}H{2}\x1b[u", (x - 1) / 2 * 2 + 1, y, p).unwrap();
                    stdout.flush().unwrap();
                }

                if b == 2 || b == 34 {
                    let p = "  ";

                    write!(stdout, "\x1b[s\x1b[{1};{0}H{2}\x1b[u", (x - 1) / 2 * 2 + 1, y, p).unwrap();
                    stdout.flush().unwrap();
                }

                state = 0;
            }
            _ => state = 0,
        }
    }
}

