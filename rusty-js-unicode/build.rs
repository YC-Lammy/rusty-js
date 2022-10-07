use std::{env, fs::File, io::Write, path::Path};

fn main() {
    let PropertyValueAliases = include_str!("PropertyValueAliases.txt").lines();

    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("PropertyValueAliases.rs");
    let mut f = File::create(&dst).unwrap();

    f.write(
        r#"
/// \[(Property, \[Value aliases])]
pub const PropetyValueAliases:&'static [(&'static str, &'static [&'static str])] = &[
    "#
        .as_bytes(),
    )
    .unwrap();

    for line in PropertyValueAliases {
        if line.starts_with('#') {
            continue;
        }
        let v = line
            .split(';')
            .map(|v| v.replace(" ", ""))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if v.len() == 0 {
            continue;
        }
        let values = &v[1..];
        f.write(
            format!(
                "    (\"{}\", &[\"{}\"]),\n",
                v[0].as_str(),
                values.join("\",\"")
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write(b"];\n").unwrap();

    let BidiBrackets = include_str!("BidiBrackets.txt").lines();

    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("BidiBrackets.rs");
    let mut f = File::create(&dst).unwrap();

    f.write(
        r#"
/// \[(code point value, Bidi_Paired_Bracket property value, open)]
pub const BidiBrackets:&'static [(char, char, bool)] = &[
    "#
        .as_bytes(),
    )
    .unwrap();

    for line in BidiBrackets {
        if line.starts_with('#') {
            continue;
        }
        let v = line
            .split('#')
            .nth(0)
            .unwrap()
            .split(';')
            .map(|v| v.replace(" ", "").replace("<none>", "0000"))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if v.len() == 0 {
            continue;
        }
        f.write(
            format!(
                "    ('\\u{{{}}}', '\\u{{{}}}', {}),\n",
                v[0].as_str(),
                v[1].as_str(),
                v[2] == "o"
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write(b"];\n").unwrap();

    let BidiMirror = include_str!("BidiMirror.txt").lines();

    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("BidiMirror.rs");
    let mut f = File::create(&dst).unwrap();

    f.write(
        r#"
/// \[(code point value, mirror value)]
pub const BidiMirrors:&'static [(char, char)] = &[
    "#
        .as_bytes(),
    )
    .unwrap();

    for line in BidiMirror {
        if line.starts_with('#') {
            continue;
        }
        let v = line
            .split('#')
            .nth(0)
            .unwrap()
            .split(';')
            .map(|v| v.replace(" ", "").replace("<none>", "0000"))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if v.len() == 0 {
            continue;
        }
        f.write(
            format!(
                "    ('\\u{{{}}}', '\\u{{{}}}'),\n",
                v[0].as_str(),
                v[1].as_str()
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write(b"];\n").unwrap();

    let Blocks = include_str!("Blocks.txt").lines();

    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("Blocks.rs");
    let mut f = File::create(&dst).unwrap();

    f.write(
        r#"
/// \[name, Range<char>]
pub const Blocks:&'static [(&'static str, std::ops::Range<u32>)] = &[
    "#
        .as_bytes(),
    )
    .unwrap();

    for line in Blocks {
        if line.starts_with('#') {
            continue;
        }
        let v = line
            .split('#')
            .nth(0)
            .unwrap()
            .split(';')
            .map(|v| v.replace(" ", "").replace("<none>", "0000"))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if v.len() == 0 {
            continue;
        }
        let r = v[0].split("..").collect::<Vec<_>>();
        f.write(format!("    (\"{}\", 0x{}..0x{}),\n", v[1].as_str(), r[0], r[1]).as_bytes())
            .unwrap();
    }
    f.write(b"];\n").unwrap();

    let CJKRadicals = include_str!("CJKRadicals.txt").lines();

    let dst = Path::new(&env::var("OUT_DIR").unwrap()).join("CJKRadicals.rs");
    let mut f = File::create(&dst).unwrap();

    f.write(
        r#"
/// \[CJK radical number, CJK radical character, CJK unified ideograph]
pub const CJKRadical:&'static [(f32, char, char)] = &[
    "#
        .as_bytes(),
    )
    .unwrap();

    for line in CJKRadicals {
        if line.starts_with('#') {
            continue;
        }
        let v = line
            .split('#')
            .nth(0)
            .unwrap()
            .split(';')
            .map(|v| v.replace(" ", "").replace("<none>", "0000"))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if v.len() == 0 {
            continue;
        }

        let radical_num = if v[0].ends_with("'") {
            let f = v[0][0..v[0].len() - 1].parse::<u8>().unwrap() as f32;
            f + 0.1
        } else {
            v[0].parse::<u8>().unwrap() as f32
        };

        f.write(
            format!(
                "    ({:.4}, '\\u{{{}}}', '\\u{{{}}}'),\n",
                radical_num, v[1], v[2]
            )
            .as_bytes(),
        )
        .unwrap();
    }
    f.write(b"];\n").unwrap();
}
