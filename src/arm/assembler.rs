#![crate_type = "cdylib"]
use regex::*;

struct Action {
    base: u32,
}

impl Action {
    fn new(base: u32) -> Action {
        Action { base }
    }

    fn substitute(&self, caps: &Captures) -> u32 {
        let rd = caps.name("rd").map_or(0, |m| Self::reg(m.as_str()));
        let rn = caps.name("rn").map_or(0, |m| Self::reg(m.as_str()));
        let rm = caps.name("rm").map_or(0, |m| Self::reg(m.as_str()));
        let rd2 = caps.name("rd2").map_or(0, |m| Self::reg(m.as_str()));
        
        let imm: u32 = caps.name("imm").map_or(0, |m| m.as_str().parse().unwrap());
        let ofs: u32 = caps.name("ofs").map_or(0, |m| m.as_str().parse().unwrap());
        // of7 can be negative, here we only consider the positive option
        let of7: u32 = caps.name("of7").map_or(0, |m| m.as_str().parse().unwrap());
        
        assert!(imm < 4096);
        assert!((ofs & 7 == 0) && (ofs < 32768));
        assert!((of7 & 7 == 0) && (of7 <= 504));
        
        let bits = rd | (rn << 5) | (rd2 << 10) | (rm << 16) | 
                    (imm << 10) | (ofs << 7) | (of7 << 12);
        
        if bits & self.base != 0 {
            panic!("invalid instruction: incursion into the base!");
        }
        
        self.base | bits
    }
    
    fn reg(s: &str) -> u32 {
        let r: u32 = s.parse().unwrap();
        assert!(r < 32);
        r
    }
}

struct Rule {
    re: Regex,
    action: Action,
}

impl Rule {
    fn new(base: u32, pat: &str) -> Rule {
        let re = Regex::new(Self::normalize(pat).as_str()).unwrap();
        Rule { re, action: Action::new(base) }
    }

    fn normalize(s: &str) -> String {
        let mut s = s.to_string();
        s.retain(|c| c != ' ');
        s.make_ascii_lowercase();
        s = s
            .replace("lr", "x30")
            .replace("sp", "x31")
            .replace("xzr", "x31");
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
            let _ = writeln!(f, "{}", s);
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
            Rule::new(0x1e604000, r"fmov d(?<rd>[0-9]+), d(?<rn>[0-9]+)"),
            Rule::new(0xaa0003e0, r"mov x(?<rd>[0-9]+), x(?<rm>[0-9]+)"),
            
            Rule::new(0xfd400000, r"ldr d(?<rd>[0-9]+), \[x(?<rn>[0-9]+), #(?<ofs>[0-9]+)\]"),
            Rule::new(0xf9400000, r"ldr x(?<rd>[0-9]+), \[x(?<rn>[0-9]+), #(?<ofs>[0-9]+)\]"),
            Rule::new(0xfd000000, r"str d(?<rd>[0-9]+), \[x(?<rn>[0-9]+), #(?<ofs>[0-9]+)\]"),
            Rule::new(0xf9000000, r"str x(?<rd>[0-9]+), \[x(?<rn>[0-9]+), #(?<ofs>[0-9]+)\]"),
            
            Rule::new(0x6d400000, r"ldp d(?<rd>[0-9]+), d(?<rd2>[0-9]+), \[x(?<rn>[0-9]+), #(?<of7>[0-9]+)\]"),
            Rule::new(0xa9400000, r"ldp x(?<rd>[0-9]+), x(?<rd2>[0-9]+), \[x(?<rn>[0-9]+), #(?<of7>[0-9]+)\]"),
            Rule::new(0x6d000000, r"stp d(?<rd>[0-9]+), d(?<rd2>[0-9]+), \[x(?<rn>[0-9]+), #(?<of7>[0-9]+)\]"),
            Rule::new(0xa9000000, r"stp x(?<rd>[0-9]+), x(?<rd2>[0-9]+), \[x(?<rn>[0-9]+), #(?<of7>[0-9]+)\]"),
            
            Rule::new(0xd1000000, r"sub x(?<rd>[0-9]+), x(?<rn>[0-9]+), #(?<imm>[0-9]+)"),
            Rule::new(0x91000000, r"add x(?<rd>[0-9]+), x(?<rn>[0-9]+), #(?<imm>[0-9]+)"),
            
            Rule::new(0x1e602800, r"fadd d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            Rule::new(0x1e603800, r"fsub d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            Rule::new(0x1e600800, r"fmul d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            Rule::new(0x1e601800, r"fdiv d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            
            Rule::new(0x1e61c000, r"fsqrt d(?<rd>[0-7]+), d(?<rn>[0-7]+)"),
            Rule::new(0x1e614000, r"fneg d(?<rd>[0-7]+), d(?<rn>[0-7]+)"),
            
            Rule::new(0x0e201c00, r"and v(?<rd>[0-9]+).8b, v(?<rn>[0-9]+).8b, v(?<rm>[0-9]+).8b"),
            Rule::new(0x0ea01c00, r"orr v(?<rd>[0-9]+).8b, v(?<rn>[0-9]+).8b, v(?<rm>[0-9]+).8b"),
            Rule::new(0x2e201c00, r"eor v(?<rd>[0-9]+).8b, v(?<rn>[0-9]+).8b, v(?<rm>[0-9]+).8b"),
            Rule::new(0x2e205800, r"not v(?<rd>[0-9]+).8b, v(?<rn>[0-9]+).8b"),
            
            Rule::new(0x5e60e400, r"fcmeq d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            Rule::new(0x7ee0e400, r"fcmlt d(?<rd>[0-9]+), d(?<rm>[0-9]+), d(?<rn>[0-9]+)"),
            Rule::new(0x7e60e400, r"fcmle d(?<rd>[0-9]+), d(?<rm>[0-9]+), d(?<rn>[0-9]+)"),
            Rule::new(0x7ee0e400, r"fcmgt d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            Rule::new(0x7e60e400, r"fcmge d(?<rd>[0-9]+), d(?<rn>[0-9]+), d(?<rm>[0-9]+)"),
            
            Rule::new(0xd63f0000, r"blr x(?<rn>[0-9]+)"),
            Rule::new(0xd65f03c0, r"ret"),
            
            Rule::new(0x9e6703e0, r"fmov d(?<rd>[0-9]+), #0.0"),
            Rule::new(0x1e6e1000, r"fmov d(?<rd>[0-9]+), #1.0"),
            Rule::new(0x1e7e1000, r"fmov d(?<rd>[0-9]+), #-1.0"),
        ];

        Assembler {
            machine_code: Vec::new(),
            rules,
            assembly: Vec::new(),
        }
    }

    pub fn translate(&self, s: &str) -> Vec<u8> {
        println!("{}", s);
        let s = Rule::normalize(s);

        for rule in self.rules.iter() {
            if rule.re.is_match(s.as_str()) {
                if let Some(caps) = rule.re.captures(s.as_str()) {
                    let x = rule.action.substitute(&caps);
                    let mut v: Vec<u8> = Vec::new();
                    v.push(x as u8);
                    v.push((x >> 8) as u8);
                    v.push((x >> 16) as u8);
                    v.push((x >> 24) as u8);
                    return v;
                }
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
}

fn main() {
    let mut a = Assembler::new();
    a.push("sub sp, sp, #32");
    println!("{:x?}", a.code());
}

#[test]
fn test_arm() {
    let mut a = Assembler::new();
    
    assert_eq!(a.translate("sub sp, sp, #32"), vec![0xFF, 0x83, 0x00, 0xD1]);
    assert_eq!(a.translate("str x29, [sp, #8]"), vec![0xFD, 0x07, 0x00, 0xF9]);
    assert_eq!(a.translate("str x30, [sp, #16]"), vec![0xFE, 0x0B, 0x00, 0xF9]);
    assert_eq!(a.translate("str d8, [sp, #24]"), vec![0xE8, 0x0F, 0x00, 0xFD]);
    assert_eq!(a.translate("mov x29, x0"), vec![0xFD, 0x03, 0x00, 0xAA]);
    
    assert_eq!(a.translate("stp x29, x30, [sp, #16]"), vec![0xFD, 0x7B, 0x01, 0xA9]);
    assert_eq!(a.translate("stp d8, d9, [sp, #160]"), vec![0xE8, 0x27, 0x0A, 0x6D]);
    assert_eq!(a.translate("ldp x19, x20, [sp, #504]"), vec![0xF3, 0xD3, 0x5F, 0xA9]);
    assert_eq!(a.translate("ldp d12, d13, [sp, #160]"), vec![0xEC, 0x37, 0x4A, 0x6D]);
    
    assert_eq!(a.translate("ldr d0, [x29, #104]"), vec![0xA0, 0x37, 0x40, 0xFD]);
    assert_eq!(a.translate("fmov d1, d0"), vec![0x01, 0x40, 0x60, 0x1E]);
    assert_eq!(a.translate("fadd d0, d0, d1"), vec![0x00, 0x28, 0x61, 0x1E]);
    assert_eq!(a.translate("fmul d0, d0, d1"), vec![0x00, 0x08, 0x61, 0x1E]);
    assert_eq!(a.translate("fsub d0, d0, d1"), vec![0x00, 0x38, 0x61, 0x1E]);
    
    assert_eq!(a.translate("fcmeq d10, d21, d9]"), vec![0xAA, 0xE6, 0x69, 0x5E]);
    assert_eq!(a.translate("fcmlt d11, d1, d19]"), vec![0x6B, 0xE6, 0xE1, 0x7E]);
    assert_eq!(a.translate("fcmle d0, d11, d31]"), vec![0xE0, 0xE7, 0x6B, 0x7E]);
    assert_eq!(a.translate("fcmgt d0, d12, d19]"), vec![0x80, 0xE5, 0xF3, 0x7E]);
    assert_eq!(a.translate("fcmge d17, d30, d3]"), vec![0xD1, 0xE7, 0x63, 0x7E]);
    
    assert_eq!(a.translate("fdiv d0, d0, d1"), vec![0x00, 0x18, 0x61, 0x1E]);
    assert_eq!(a.translate("str d0, [x30, #200]"), vec![0xC0, 0x67, 0x00, 0xFD]);
    assert_eq!(a.translate("ldr x29, [sp, #8]"), vec![0xFD, 0x07, 0x40, 0xF9]);
    assert_eq!(a.translate("ldr x30, [sp, #16]"), vec![0xFE, 0x0B, 0x40, 0xF9]);
    assert_eq!(a.translate("add sp, sp, #32"), vec![0xFF, 0x83, 0x00, 0x91]);
    
    assert_eq!(a.translate("and v2.8b, v5.8b, v22.8b"), vec![0xA2, 0x1C, 0x36, 0x0E]);
    assert_eq!(a.translate("orr v1.8b, v0.8b, v12.8b"), vec![0x01, 0x1C, 0xAC, 0x0E]);
    assert_eq!(a.translate("eor v7.8b, v15.8b, v31.8b"), vec![0xE7, 0x1D, 0x3F, 0x2E]);
    assert_eq!(a.translate("not v14.8b, v24.8b"), vec![0x0E, 0x5B, 0x20, 0x2E]);
    
    assert_eq!(a.translate("ldr lr, [sp, #1000]"), vec![0xFE, 0xF7, 0x41, 0xF9]);
    assert_eq!(a.translate("str lr, [sp, #2000]"), vec![0xFE, 0xEB, 0x03, 0xF9]);
    assert_eq!(a.translate("blr x6"), vec![0xC0, 0x00, 0x3F, 0xD6]);
    assert_eq!(a.translate("ret"), vec![0xC0, 0x03, 0x5F, 0xD6]);
    
    println!("{:x?}", a.code());
}

