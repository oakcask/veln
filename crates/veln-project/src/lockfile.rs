use std::fs;
use std::io;
use std::path::Path;

use crate::discover_source_paths;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectLockfile {
    pub packages: Vec<LockfilePackage>,
}

impl ProjectLockfile {
    pub fn render(&self) -> String {
        let mut packages = self.packages.clone();
        packages.sort_by(|left, right| left.name.cmp(&right.name));

        let mut out = String::new();
        for (index, package) in packages.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str("[[package]]\n");
            out.push_str(&format!(
                "name = \"{}\"\n",
                escape_toml_string(&package.name)
            ));
            out.push_str(&format!("source = {}\n", package.source.render()));
            out.push_str(&format!(
                "checksum = \"{}\"\n",
                escape_toml_string(&package.checksum)
            ));
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockfilePackage {
    pub name: String,
    pub source: LockfileSource,
    pub checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LockfileSource {
    Path {
        path: String,
    },
    Git {
        url: String,
        selector: LockfileGitSelector,
        rev: String,
        subdir: Option<String>,
    },
}

impl LockfileSource {
    fn render(&self) -> String {
        match self {
            Self::Path { path } => {
                format!(
                    "{{ kind = \"path\", path = \"{}\" }}",
                    escape_toml_string(path)
                )
            }
            Self::Git {
                url,
                selector,
                rev,
                subdir,
            } => {
                let mut fields = vec![
                    "kind = \"git\"".to_string(),
                    format!("url = \"{}\"", escape_toml_string(url)),
                    format!("selector = {}", selector.render()),
                    format!("rev = \"{}\"", escape_toml_string(rev)),
                ];
                if let Some(subdir) = subdir {
                    fields.push(format!("subdir = \"{}\"", escape_toml_string(subdir)));
                }
                format!("{{ {} }}", fields.join(", "))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LockfileGitSelector {
    Rev(String),
    Tag(String),
    Branch(String),
}

impl LockfileGitSelector {
    fn render(&self) -> String {
        match self {
            Self::Rev(value) => format!("{{ rev = \"{}\" }}", escape_toml_string(value)),
            Self::Tag(value) => format!("{{ tag = \"{}\" }}", escape_toml_string(value)),
            Self::Branch(value) => {
                format!("{{ branch = \"{}\" }}", escape_toml_string(value))
            }
        }
    }
}

pub fn source_tree_checksum(root: &Path) -> io::Result<String> {
    let paths = discover_source_paths(root, &[])?;
    let mut entries = paths
        .into_iter()
        .map(|path| (package_relative_path(root, &path), path))
        .collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut hash = Sha256::new();
    hash.update(b"veln-source-tree/v1\n");
    for (relative, path) in entries {
        let contents = fs::read(&path)?;
        hash.update(&(relative.len() as u64).to_be_bytes());
        hash.update(relative.as_bytes());
        hash.update(&(contents.len() as u64).to_be_bytes());
        hash.update(&contents);
    }
    Ok(format!("sha256:{}", hex(&hash.finalize())))
}

pub fn write_lockfile(root: &Path, lockfile: &ProjectLockfile) -> io::Result<()> {
    fs::write(root.join("veln.lock"), lockfile.render())
}

pub fn normalize_lockfile_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn package_relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    path_to_lockfile_string(relative)
}

fn path_to_lockfile_string(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        parts.push(component.as_os_str().to_string_lossy());
    }
    parts.join("/")
}

fn escape_toml_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    length_bits: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            length_bits: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.length_bits = self.length_bits.wrapping_add((bytes.len() as u64) * 8);
        if self.buffer_len > 0 {
            let copied = (64 - self.buffer_len).min(bytes.len());
            self.buffer[self.buffer_len..self.buffer_len + copied]
                .copy_from_slice(&bytes[..copied]);
            self.buffer_len += copied;
            bytes = &bytes[copied..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            } else {
                return;
            }
        }
        while bytes.len() >= 64 {
            let block: [u8; 64] = bytes[..64].try_into().expect("block length should match");
            self.compress(&block);
            bytes = &bytes[64..];
        }
        self.buffer[..bytes.len()].copy_from_slice(bytes);
        self.buffer_len = bytes.len();
    }

    fn finalize(mut self) -> [u8; 32] {
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            let block = self.buffer;
            self.compress(&block);
            self.buffer_len = 0;
        }
        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..].copy_from_slice(&self.length_bits.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);

        let mut out = [0; 32];
        for (chunk, word) in out.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes(chunk.try_into().expect("chunk length should match"));
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for word in w.iter().zip(K) {
            let (word, constant) = word;
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(ch)
                .wrapping_add(constant)
                .wrapping_add(*word);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (state, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
}
