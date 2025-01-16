use regex::*;

struct Rule {
    re: Regex,
    action: fn(&Captures) -> Vec<u8>,
}

impl Rule {
    fn new(pat: &str, action: fn(&Captures) -> Vec<u8>) -> Rule {
        let re = Regex::new(Self::normalize(pat).as_str()).unwrap();
        Rule { re, action }
    }

    fn normalize(s: &str) -> String {
        let mut s = s.to_string();
        s.retain(|c| c != ' ');
        s.make_ascii_lowercase();
        s
    }
}

pub struct Assembler {
    machine_code: Vec<u8>,
    rules: Vec<Rule>,
    assembly: Vec<String>,
}

impl std::fmt::Display for Assembler {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for s in self.assembly.iter() {
            writeln!(f, "{}", s);
        }
        Ok(())
    }
}

impl std::fmt::Debug for Assembler {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl Assembler {
    pub fn new() -> Assembler {
        let rules = vec![
            Rule::new(r"movsd xmm([0-7]), xmm([0-7])", Self::movsd_xmm_xmm),
            Rule::new(
                r"movsd xmm([0-7]), qword ptr \[r([a-z0-9]+)\+0x([0-9a-f]+)\]",
                Self::movsd_xmm_mem,
            ),
            Rule::new(
                r"movsd qword ptr \[r([a-z0-9]+)\+0x([0-9a-f]+)\], xmm([0-7])",
                Self::movsd_mem_xmm,
            ),
            Rule::new(r"movq xmm([0-7]), r([a-z0-9]+)", Self::movq_xmm_reg),
            Rule::new(r"movq r([a-z0-9]+), xmm([0-7])", Self::movq_reg_xmm),
            Rule::new(r"mov r([a-z0-9]+), r([a-z0-9]+)", Self::mov_reg_reg),
            Rule::new(
                r"mov r([a-z0-9]+), qword ptr \[r([a-z0-9]+)\+0x([0-9a-f]+)\]",
                Self::mov_reg_mem,
            ),
            Rule::new(
                r"mov qword ptr \[r([a-z0-9]+)\+0x([0-9a-f]+)\], r([a-z0-9]+)",
                Self::mov_mem_reg,
            ),
            Rule::new(r"addsd xmm([0-7]), xmm([0-7])", Self::addsd_xmm_xmm),
            Rule::new(r"subsd xmm([0-7]), xmm([0-7])", Self::subsd_xmm_xmm),
            Rule::new(r"mulsd xmm([0-7]), xmm([0-7])", Self::mulsd_xmm_xmm),
            Rule::new(r"divsd xmm([0-7]), xmm([0-7])", Self::divsd_xmm_xmm),
            Rule::new(r"sqrtsd xmm([0-7]), xmm([0-7])", Self::sqrtsd_xmm_xmm),
            Rule::new(r"rsqrtsd xmm([0-7]), xmm([0-7])", Self::rsqrtsd_xmm_xmm),
            Rule::new(r"andpd xmm([0-7]), xmm([0-7])", Self::andpd_xmm_xmm),
            Rule::new(r"andnpd xmm([0-7]), xmm([0-7])", Self::andnpd_xmm_xmm),
            Rule::new(r"^orpd xmm([0-7]), xmm([0-7])", Self::orpd_xmm_xmm),
            Rule::new(r"xorpd xmm([0-7]), xmm([0-7])", Self::xorpd_xmm_xmm),
            Rule::new(r"call r([a-z0-9]+)", Self::call_reg),
            Rule::new(r"push r([a-z0-9]+)", Self::push_reg),
            Rule::new(r"pop r([a-z0-9]+)", Self::pop_reg),
            Rule::new(r"ret", Self::ret),
            Rule::new(r"cmp([^s]+)sd xmm([0-7]), xmm([0-7])", Self::cmp_xmm_xmm),
        ];

        Assembler {
            machine_code: Vec::new(),
            rules,
            assembly: Vec::new(),
        }
    }

    pub fn translate(&self, s: &str) -> Vec<u8> {
        let s = Rule::normalize(s);

        for rule in self.rules.iter() {
            if let Some(caps) = rule.re.captures(s.as_str()) {
                return (rule.action)(&caps);
            }
        }

        panic!("unrecognized instruction: {:?}", s);
    }

    pub fn push(&mut self, s: &str) {
        let mut b = self.translate(s);
        self.machine_code.append(&mut b);
        self.assembly.push(s.to_string());
    }

    pub fn code(&mut self) -> Vec<u8> {
        self.machine_code.clone()
    }

    fn modrm_reg(dst: u8, src: u8) -> u8 {
        0xC0 + (src << 3) + dst
    }

    fn modrm_mem(dst: u8, base: u8, offset: usize) -> Vec<u8> {
        if offset < 128 {
            // note: disp8 is 2's complement
            vec![0x40 + (dst << 3) + base, offset as u8]
        } else {
            vec![
                0x80 + (dst << 3) + base,
                offset as u8,
                (offset >> 8) as u8,
                (offset >> 16) as u8,
                (offset >> 24) as u8,
            ]
        }
    }

    fn xmm(s: &str) -> u8 {
        u8::from_str_radix(s, 10).unwrap()
    }

    fn reg(s: &str) -> u8 {
        match s {
            "ax" => 0,
            "cx" => 1,
            "dx" => 2,
            "bx" => 3,
            "sp" => 4,
            "bp" => 5,
            "si" => 6,
            "di" => 7,
            "8" => 8,
            "9" => 9,
            "10" => 10,
            "11" => 11,
            "12" => 12,
            "13" => 13,
            "14" => 14,
            "15" => 15,
            _ => {
                panic!("invalid gp register");
            }
        }
    }

    fn movsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x10, Self::modrm_reg(src, dst)]
    }

    fn movsd_xmm_mem(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let base = Self::reg(&caps[2]);
        let offset = usize::from_str_radix(&caps[3], 16).unwrap();
        let mut v = vec![0xf2, 0x0f, 0x10];
        v.extend_from_slice(&Self::modrm_mem(dst, base, offset)[..]);
        v
    }

    fn movsd_mem_xmm(caps: &Captures) -> Vec<u8> {
        let base = Self::reg(&caps[1]);
        let offset = usize::from_str_radix(&caps[2], 16).unwrap();
        let src = Self::xmm(&caps[3]);
        let mut v = vec![0xf2, 0x0f, 0x11];
        v.extend_from_slice(&Self::modrm_mem(src, base, offset)[..]);
        v
    }

    fn movq_xmm_reg(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::reg(&caps[2]);
        vec![
            0x66,
            0x48 | (src >> 3),
            0x0f,
            0x6e,
            Self::modrm_reg(src & 7, dst),
        ]
    }

    fn movq_reg_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::reg(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![
            0x66,
            0x48 | (dst >> 3),
            0x0f,
            0x7e,
            Self::modrm_reg(dst & 7, src),
        ]
    }

    fn mov_reg_reg(caps: &Captures) -> Vec<u8> {
        let dst = Self::reg(&caps[1]);
        let src = Self::reg(&caps[2]);
        vec![0x48, 0x89, Self::modrm_reg(dst, src)]
    }

    fn mov_reg_mem(caps: &Captures) -> Vec<u8> {
        let dst = Self::reg(&caps[1]);
        let base = Self::reg(&caps[2]);
        let offset = usize::from_str_radix(&caps[3], 16).unwrap();
        let mut v = vec![0x48, 0x8b];
        v.extend_from_slice(&Self::modrm_mem(dst, base, offset)[..]);
        v
    }

    fn mov_mem_reg(caps: &Captures) -> Vec<u8> {
        let base = Self::reg(&caps[1]);
        let offset = usize::from_str_radix(&caps[2], 16).unwrap();
        let src = Self::reg(&caps[3]);
        let mut v = vec![0x48, 0x89];
        v.extend_from_slice(&Self::modrm_mem(src, base, offset)[..]);
        v
    }

    fn addsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x58, Self::modrm_reg(src, dst)]
    }

    fn subsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x5c, Self::modrm_reg(src, dst)]
    }

    fn mulsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x59, Self::modrm_reg(src, dst)]
    }

    fn divsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x5e, Self::modrm_reg(src, dst)]
    }

    fn sqrtsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x51, Self::modrm_reg(src, dst)]
    }

    fn rsqrtsd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0xf2, 0x0f, 0x52, Self::modrm_reg(src, dst)]
    }

    fn andpd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0x66, 0x0f, 0x54, Self::modrm_reg(src, dst)]
    }

    fn andnpd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0x66, 0x0f, 0x55, Self::modrm_reg(src, dst)]
    }

    fn orpd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0x66, 0x0f, 0x56, Self::modrm_reg(src, dst)]
    }

    fn xorpd_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let dst = Self::xmm(&caps[1]);
        let src = Self::xmm(&caps[2]);
        vec![0x66, 0x0f, 0x57, Self::modrm_reg(src, dst)]
    }

    fn cmp_xmm_xmm(caps: &Captures) -> Vec<u8> {
        let code = match &caps[1] {
            "eq" => 0,
            "lt" => 1,
            "le" => 2,
            "unord" => 3,
            "eq" => 4,
            "nlt" => 5,
            "nle" => 6,
            "ord" => 7,
            _ => {
                panic!("unrecognized comparison");
            }
        };
        let dst = Self::xmm(&caps[2]);
        let src = Self::xmm(&caps[3]);
        vec![0xf2, 0x0f, 0xc2, Self::modrm_reg(src, dst), code]
    }

    fn call_reg(caps: &Captures) -> Vec<u8> {
        let src = Self::reg(&caps[1]);
        vec![0xff, 0xd0 + src]
    }

    fn push_reg(caps: &Captures) -> Vec<u8> {
        let src = Self::reg(&caps[1]);
        if src < 8 {
            vec![0x50 + src]
        } else {
            vec![0x41, 0x48 + src]
        }
    }

    fn pop_reg(caps: &Captures) -> Vec<u8> {
        let dst = Self::reg(&caps[1]);
        if dst < 8 {
            vec![0x58 + dst]
        } else {
            vec![0x41, 0x50 + dst]
        }
    }

    fn ret(caps: &Captures) -> Vec<u8> {
        vec![0xc3]
    }
}

#[test]
fn test_assembler() {
    let mut a = Assembler::new();

    assert_eq!(vec![0x55], a.translate("push rbp"));
    assert_eq!(vec![0x53], a.translate("push rbx"));
    assert_eq!(vec![0x48, 0x89, 0xfd], a.translate("mov rbp,rdi"));
    assert_eq!(
        vec![0xf2, 0x0f, 0x10, 0x45, 0x58],
        a.translate("movsd xmm0,QWORD PTR [rbp+0x58]")
    );
    assert_eq!(
        vec![0xf2, 0x0f, 0x11, 0x85, 0xf8, 0x00, 0x00, 0x00],
        a.translate("movsd QWORD PTR [rbp+0xf8],xmm0")
    );
    assert_eq!(vec![0xf2, 0x0f, 0x59, 0xc1], a.translate("mulsd xmm0,xmm1"));
    assert_eq!(vec![0xf2, 0x0f, 0x5e, 0xc1], a.translate("divsd xmm0,xmm1"));
    assert_eq!(
        vec![0x48, 0x8b, 0x43, 0x10],
        a.translate("mov rax,QWORD PTR [rbx+0x10]")
    );
    assert_eq!(
        vec![0x48, 0x8b, 0x9b, 0x34, 0x12, 0x00, 0x00],
        a.translate("mov rbx,QWORD PTR [rbx+0x1234]")
    );
    assert_eq!(vec![0xff, 0xd0], a.translate("call rax"));
    assert_eq!(vec![0x66, 0x0f, 0x57, 0xc1], a.translate("xorpd xmm0,xmm1"));
    assert_eq!(
        vec![0xf2, 0x0f, 0xc2, 0xc1, 0x05],
        a.translate("cmpnltsd xmm0,xmm1")
    );
    assert_eq!(
        vec![0x66, 0x0f, 0x55, 0xd9],
        a.translate("andnpd xmm3,xmm1")
    );
    assert_eq!(vec![0x66, 0x0f, 0x54, 0xe2], a.translate("andpd xmm4,xmm2"));
    assert_eq!(
        vec![0xf2, 0x0f, 0x10, 0x4d, 0x18],
        a.translate("movsd  xmm1,QWORD PTR [rbp+0x18]")
    );
    assert_eq!(vec![0x66, 0x0f, 0x56, 0xe5], a.translate("orpd  xmm4,xmm5"));
    assert_eq!(
        vec![0x66, 0x0f, 0x57, 0xc1],
        a.translate("xorpd  xmm0,xmm1")
    );
    assert_eq!(
        vec![0xf2, 0x0f, 0x10, 0xcc],
        a.translate("movsd  xmm1,xmm4")
    );
    assert_eq!(
        vec![0xf2, 0x0f, 0x58, 0xc1],
        a.translate("addsd  xmm0,xmm1")
    );
    assert_eq!(
        vec![0xf2, 0x0f, 0x10, 0xcd],
        a.translate("movsd  xmm1,xmm5")
    );
    assert_eq!(
        vec![0x66, 0x48, 0x0f, 0x7e, 0xde],
        a.translate("movq  rsi,xmm3")
    );
    assert_eq!(
        vec![0x66, 0x48, 0x0f, 0x6e, 0xe9],
        a.translate("movq  xmm5,rcx")
    );
    assert_eq!(vec![0x5d], a.translate("pop rbp"));
    assert_eq!(vec![0xc3], a.translate("ret"));
}
