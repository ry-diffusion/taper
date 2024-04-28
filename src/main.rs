use std::{
    collections::HashMap,
    env::args,
    fs::{self, File},
    io::{stdout, BufRead, BufReader, Read, Result, Write},
    sync::OnceLock,
};

const CONVERT_TABLE: [[char; 5]; 5] = [
    ['A', 'B', 'C', 'D', 'E'],
    ['F', 'G', 'H', 'I', 'J'],
    ['L', 'M', 'N', 'O', 'P'],
    ['Q', 'R', 'S', 'T', 'U'],
    ['V', 'W', 'X', 'Y', 'Z'],
];

static LOOKUP_TABLE: OnceLock<HashMap<char, (usize, usize)>> = OnceLock::new();

#[derive(Debug, Clone)]
enum DecoderState {
    IndexingColumn,
    IndexingRow,
}

fn try_read_mapping_table(path: String) -> Option<HashMap<char, char>> {
    let mut map = HashMap::new();
    let contents = fs::read_to_string(path).ok()?;

    for line in contents.lines() {
        let (key, value) = line.split_once(',')?;
        map.insert(key.chars().next()?, value.chars().next().unwrap_or('-'));
    }

    Some(map)
}

fn main() -> Result<()> {
    LOOKUP_TABLE.get_or_init(|| {
        let mut lookup_table = HashMap::<char, (usize, usize)>::new();

        for (i, column) in CONVERT_TABLE.iter().enumerate() {
            for (j, row) in column.iter().enumerate() {
                lookup_table.insert(*row, (i + 1, 1 + j));
            }
        }

        lookup_table
    });

    let mut args = args();

    args.next();
    let mode_owned = args.next();

    let Some(mode @ ("decode" | "encode")) = mode_owned.as_deref() else {
        panic!("eu preciso de um modo válido!");
    };

    let Some(input_path) = args.next() else {
        panic!("ei! eu preciso de um arquivo de input..");
    };

    let Some(output_path) = args.next() else {
        panic!("ei! eu preciso de um arquivo de output");
    };

    eprintln!("[*] input: {input_path}");
    eprintln!("[*] output: {output_path}");
    eprintln!("[*] mode: {mode}");

    let mut input = File::open(input_path)?;
    let mut output: Box<dyn Write> = if output_path == "-" {
        Box::new(stdout())
    } else {
        Box::new(File::create(output_path)?)
    };
    let map_table = args
        .next()
        .and_then(try_read_mapping_table)
        .unwrap_or_default();

    match mode {
        "encode" => encode_to(&mut input, &mut output, &map_table)?,
        "decode" => decode_to(&mut input, &mut output)?,
        _ => {}
    }
    Ok(())
}

fn encode_to<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
    map_table: &HashMap<char, char>,
) -> Result<()> {
    let reader = BufReader::new(input);

    let lookup_table = LOOKUP_TABLE.get().expect("lookup table not init yet.");

    for line in reader.lines().map_while(Result::ok) {
        for mut character in line.chars() {
            character.make_ascii_uppercase();

            if let Some(mapped_character) = map_table.get(&character) {
                character = mapped_character.to_ascii_uppercase();
            }

            if character == '-' {
                continue;
            }

            let Some((i, j)) = lookup_table.get(&character) else {
                eprintln!("[!] unsupported: '{character}'");
                continue;
            };

            let i = ".".repeat(*i);
            let j = ".".repeat(*j);

            write!(output, "{i} {j} ")?;
        }

        writeln!(output)?;
    }

    Ok(())
}

fn decode_to<R: Read, W: Write>(input: &mut R, output: &mut W) -> Result<()> {
    let reader = BufReader::new(input);
    let mut state = DecoderState::IndexingColumn;

    for line in reader.lines().map_while(Result::ok) {
        let (mut i, mut j) = (0, 0);

        for chr in line.chars() {
            use DecoderState::{IndexingColumn, IndexingRow};

            match state {
                IndexingColumn if chr == '.' => i += 1,
                IndexingRow if chr == '.' => j += 1,

                IndexingColumn if chr == ' ' => state = IndexingRow,
                IndexingRow if chr == ' ' => {
                    let col = if i == 0 { 1 } else { i - 1 };
                    let idx = if j == 0 { 1 } else { j - 1 };

                    if let Some(chr) = CONVERT_TABLE.get(col).and_then(|row| row.get(idx)) {
                        write!(output, "{chr}")?;
                    }

                    (i, j) = (0, 0);
                    state = IndexingColumn;
                }
                _ => {}
            }
        }

        writeln!(output)?;
    }

    Ok(())
}
