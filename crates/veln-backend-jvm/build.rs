use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use unicode_normalization::char::{canonical_combining_class, compose, decompose_canonical};

const RUNTIME_FRAGMENTS: &[&str] = &[
    "values.java.inc",
    "concurrency.java.inc",
    "observability.java.inc",
    "effects.java.inc",
    "accessors.java.inc",
    "collections.java.inc",
    "option_result.java.inc",
    "operators.java.inc",
    "stdio.java.inc",
    "diagnostics.java.inc",
    "interop.java.inc",
];

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let runtime_source_dir = manifest_dir.join("src").join("runtime");
    let runtime_source = runtime_source(&runtime_source_dir);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let runtime_dir = out_dir.join("runtime");
    if runtime_dir.exists() {
        fs::remove_dir_all(&runtime_dir).expect("failed to remove stale generated runtime classes; check build output permissions before retrying so removed runtime implementations cannot remain in the bundle");
    }
    fs::create_dir_all(&runtime_dir).expect("runtime output dir should be created");
    fs::write(runtime_dir.join("VelnRuntime.java"), runtime_source)
        .expect("runtime source should be written");

    let output = Command::new("javac")
        .arg("--release")
        .arg("8")
        .arg("VelnRuntime.java")
        .current_dir(&runtime_dir)
        .output()
        .expect("javac should run; install a JDK to build veln-backend-jvm");
    if !output.status.success() {
        panic!(
            "javac failed while building JVM runtime classes:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    write_runtime_class_manifest(&out_dir, &runtime_dir);
}

fn runtime_source(runtime_source_dir: &Path) -> String {
    let mut source = String::from("public final class VelnRuntime {\n");
    for fragment in RUNTIME_FRAGMENTS {
        let path = runtime_source_dir.join(fragment);
        println!("cargo:rerun-if-changed=src/runtime/{fragment}");
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read runtime fragment `{fragment}`: {error}")
        });
        source.push_str(&text);
        if !text.ends_with('\n') {
            source.push('\n');
        }
    }
    source.push_str(&portable_unicode_source());
    source.push_str("}\n");
    source
}

fn portable_unicode_source() -> String {
    assert_eq!(unicode_normalization::UNICODE_VERSION, (17, 0, 0));

    let mut decompositions = Vec::new();
    let mut combining_classes = Vec::new();
    let mut compositions = BTreeMap::new();
    let mut whitespace = Vec::new();
    let mut controls = Vec::new();

    for value in 0..=0x10ffff {
        let Some(character) = char::from_u32(value) else {
            continue;
        };
        let combining_class = canonical_combining_class(character);
        if combining_class != 0 {
            combining_classes.push((value, combining_class));
        }
        if character.is_whitespace() {
            whitespace.push(value);
        }
        if character.is_control() {
            controls.push(value);
        }

        if (0xac00..0xd7a4).contains(&value) {
            continue;
        }
        let mut decomposition = Vec::new();
        decompose_canonical(character, |part| decomposition.push(part));
        if decomposition.len() == 1 && decomposition[0] == character {
            continue;
        }
        decompositions.push((value, decomposition.clone()));

        let mut starter = decomposition[0];
        let mut last_class = 0;
        for part in decomposition.into_iter().skip(1) {
            let part_class = canonical_combining_class(part);
            if (last_class == 0 || last_class < part_class)
                && let Some(composite) = compose(starter, part)
            {
                compositions.insert((starter as u32, part as u32), composite as u32);
                starter = composite;
            } else {
                if part_class == 0 {
                    starter = part;
                }
                last_class = part_class;
            }
        }
    }

    let mut bytes = Vec::new();
    push_u32(&mut bytes, decompositions.len());
    for (character, decomposition) in decompositions {
        push_u32(&mut bytes, character as usize);
        bytes.push(u8::try_from(decomposition.len()).expect("canonical decomposition length"));
        for part in decomposition {
            push_u32(&mut bytes, part as usize);
        }
    }
    push_u32(&mut bytes, combining_classes.len());
    for (character, combining_class) in combining_classes {
        push_u32(&mut bytes, character as usize);
        bytes.push(combining_class);
    }
    push_u32(&mut bytes, compositions.len());
    for ((starter, part), composite) in compositions {
        push_u32(&mut bytes, starter as usize);
        push_u32(&mut bytes, part as usize);
        push_u32(&mut bytes, composite as usize);
    }
    push_u32(&mut bytes, whitespace.len());
    for character in whitespace {
        push_u32(&mut bytes, character as usize);
    }
    push_u32(&mut bytes, controls.len());
    for character in controls {
        push_u32(&mut bytes, character as usize);
    }

    let encoded = encode_base64(&bytes);
    let mut chunks = String::from("        StringBuilder encoded = new StringBuilder();\n");
    for chunk in encoded.as_bytes().chunks(16_000) {
        writeln!(
            chunks,
            "        encoded.append({:?});",
            std::str::from_utf8(chunk).expect("base64 is UTF-8")
        )
        .unwrap();
    }

    format!(
        r#"
    private static final String PORTABLE_UNICODE_VERSION = "17.0.0";

    private static final class PortableUnicodeData {{
        final java.util.Map<Integer, int[]> decompositions = new java.util.HashMap<Integer, int[]>();
        final java.util.Map<Integer, Integer> combiningClasses = new java.util.HashMap<Integer, Integer>();
        final java.util.Map<Long, Integer> compositions = new java.util.HashMap<Long, Integer>();
        final java.util.Set<Integer> whitespace = new java.util.HashSet<Integer>();
        final java.util.Set<Integer> controls = new java.util.HashSet<Integer>();
    }}

    private static PortableUnicodeData portableUnicodeData() {{
{chunks}        java.nio.ByteBuffer input = java.nio.ByteBuffer.wrap(
            java.util.Base64.getDecoder().decode(encoded.toString()));
        PortableUnicodeData data = new PortableUnicodeData();
        int decompositionCount = input.getInt();
        for (int index = 0; index < decompositionCount; index++) {{
            int character = input.getInt();
            int length = input.get() & 0xff;
            int[] decomposition = new int[length];
            for (int part = 0; part < length; part++) decomposition[part] = input.getInt();
            data.decompositions.put(Integer.valueOf(character), decomposition);
        }}
        int combiningClassCount = input.getInt();
        for (int index = 0; index < combiningClassCount; index++) {{
            data.combiningClasses.put(Integer.valueOf(input.getInt()), Integer.valueOf(input.get() & 0xff));
        }}
        int compositionCount = input.getInt();
        for (int index = 0; index < compositionCount; index++) {{
            int starter = input.getInt();
            int part = input.getInt();
            data.compositions.put(Long.valueOf(portableUnicodeCompositionKey(starter, part)), Integer.valueOf(input.getInt()));
        }}
        int whitespaceCount = input.getInt();
        for (int index = 0; index < whitespaceCount; index++) data.whitespace.add(Integer.valueOf(input.getInt()));
        int controlCount = input.getInt();
        for (int index = 0; index < controlCount; index++) data.controls.add(Integer.valueOf(input.getInt()));
        if (input.hasRemaining()) throw new IllegalStateException("trailing portable Unicode data");
        return data;
    }}

    private static final PortableUnicodeData PORTABLE_UNICODE = portableUnicodeData();

    private static long portableUnicodeCompositionKey(int starter, int part) {{
        return ((long) starter << 21) | (long) part;
    }}

    private static int portableUnicodeCombiningClass(int character) {{
        Integer value = PORTABLE_UNICODE.combiningClasses.get(Integer.valueOf(character));
        return value == null ? 0 : value.intValue();
    }}

    private static Integer portableUnicodeCompose(int starter, int part) {{
        final int sBase = 0xac00;
        final int lBase = 0x1100;
        final int vBase = 0x1161;
        final int tBase = 0x11a7;
        final int lCount = 19;
        final int vCount = 21;
        final int tCount = 28;
        final int nCount = vCount * tCount;
        final int sCount = lCount * nCount;
        if (starter >= lBase && starter < lBase + lCount
            && part >= vBase && part < vBase + vCount) {{
            return Integer.valueOf(sBase + (starter - lBase) * nCount + (part - vBase) * tCount);
        }}
        if (starter >= sBase && starter < sBase + sCount
            && (starter - sBase) % tCount == 0
            && part > tBase && part < tBase + tCount) {{
            return Integer.valueOf(starter + part - tBase);
        }}
        return PORTABLE_UNICODE.compositions.get(
            Long.valueOf(portableUnicodeCompositionKey(starter, part)));
    }}

    private static void portableUnicodeDecompose(int character, java.util.List<Integer> output) {{
        final int sBase = 0xac00;
        final int lBase = 0x1100;
        final int vBase = 0x1161;
        final int tBase = 0x11a7;
        final int vCount = 21;
        final int tCount = 28;
        final int nCount = vCount * tCount;
        final int sCount = 19 * nCount;
        if (character >= sBase && character < sBase + sCount) {{
            int sIndex = character - sBase;
            output.add(Integer.valueOf(lBase + sIndex / nCount));
            output.add(Integer.valueOf(vBase + (sIndex % nCount) / tCount));
            int trailing = sIndex % tCount;
            if (trailing != 0) output.add(Integer.valueOf(tBase + trailing));
            return;
        }}
        int[] decomposition = PORTABLE_UNICODE.decompositions.get(Integer.valueOf(character));
        if (decomposition == null) {{
            output.add(Integer.valueOf(character));
            return;
        }}
        for (int part : decomposition) output.add(Integer.valueOf(part));
    }}

    private static boolean portableUnicodeIsNfc(String value) {{
        java.util.ArrayList<Integer> decomposed = new java.util.ArrayList<Integer>();
        for (int offset = 0; offset < value.length();) {{
            int character = value.codePointAt(offset);
            offset += Character.charCount(character);
            int before = decomposed.size();
            portableUnicodeDecompose(character, decomposed);
            for (int index = before; index < decomposed.size(); index++) {{
                int combiningClass = portableUnicodeCombiningClass(decomposed.get(index).intValue());
                if (combiningClass == 0) continue;
                int insertion = index;
                while (insertion > 0) {{
                    int priorClass = portableUnicodeCombiningClass(decomposed.get(insertion - 1).intValue());
                    if (priorClass == 0 || priorClass <= combiningClass) break;
                    java.util.Collections.swap(decomposed, insertion - 1, insertion);
                    insertion -= 1;
                }}
            }}
        }}
        if (decomposed.isEmpty()) return value.isEmpty();

        java.util.ArrayList<Integer> composed = new java.util.ArrayList<Integer>();
        int starterIndex = 0;
        int starter = decomposed.get(0).intValue();
        int lastClass = 0;
        composed.add(Integer.valueOf(starter));
        for (int index = 1; index < decomposed.size(); index++) {{
            int part = decomposed.get(index).intValue();
            int partClass = portableUnicodeCombiningClass(part);
            Integer composite = portableUnicodeCompose(starter, part);
            if (composite != null && (lastClass == 0 || lastClass < partClass)) {{
                starter = composite.intValue();
                composed.set(starterIndex, composite);
            }} else {{
                if (partClass == 0) {{
                    starterIndex = composed.size();
                    starter = part;
                }}
                composed.add(Integer.valueOf(part));
                lastClass = partClass;
            }}
        }}

        int inputOffset = 0;
        for (Integer character : composed) {{
            if (inputOffset >= value.length() || value.codePointAt(inputOffset) != character.intValue()) return false;
            inputOffset += Character.charCount(character.intValue());
        }}
        return inputOffset == value.length();
    }}
"#
    )
}

fn push_u32(output: &mut Vec<u8>, value: usize) {
    output.extend_from_slice(
        &u32::try_from(value)
            .expect("portable Unicode value")
            .to_be_bytes(),
    );
}

fn encode_base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    for bytes in input.chunks(3) {
        let value = (u32::from(bytes[0]) << 16)
            | (u32::from(*bytes.get(1).unwrap_or(&0)) << 8)
            | u32::from(*bytes.get(2).unwrap_or(&0));
        output.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
        output.push(if bytes.len() > 1 {
            ALPHABET[((value >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        output.push(if bytes.len() > 2 {
            ALPHABET[(value & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn write_runtime_class_manifest(out_dir: &Path, runtime_dir: &Path) {
    let mut class_names = fs::read_dir(runtime_dir)
        .expect("runtime output dir should be readable")
        .map(|entry| {
            let entry = entry.expect("runtime output entry should be readable");
            entry.file_name().to_string_lossy().into_owned()
        })
        .filter(|name| name.ends_with(".class"))
        .collect::<Vec<_>>();
    class_names.sort();

    if !class_names.iter().any(|name| name == "VelnRuntime.class") {
        panic!(
            "javac did not produce VelnRuntime.class; inspect the runtime fragment compile output"
        );
    }

    let mut manifest = Vec::new();
    writeln!(manifest, "const RUNTIME_CLASSES: &[(&str, &[u8])] = &[").unwrap();
    for name in class_names {
        writeln!(
            manifest,
            "    ({name:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/runtime/{name}\"))),"
        )
        .unwrap();
    }
    writeln!(manifest, "];").unwrap();

    fs::write(out_dir.join("runtime_classes.rs"), manifest)
        .expect("runtime class manifest should be written");
}
