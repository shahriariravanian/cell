#![allow(unused_parens)]
pub fn func(mem: &mut [f64]) {
    let t_31 = mem[11];
    let t_32 = mem[12];
    let t_33 = mem[10];
    let t_34 = mem[9];
    let t_35 = mem[11];
    let t_36 = (-82.3 as f64) + ((-13.0287 as f64) * (f64::ln((0.001 as f64) * (mem[7]))));
    let t_43 = mem[6];
    let t_44 = mem[5];
    let t_45 = mem[11];
    let t_46 = mem[8];
    let t_47 = mem[11];
    let t_48 = mem[11];
    let t_49 = mem[11];
    let t_50 = mem[11];
    let t_51 = mem[11];
    let t_52 = mem[11];
    let t_53 = mem[11];
    let t_54 = (if (((if ((mem[21]) - (mem[4])) >= (0 as f64) {
        (1 as f64)
    } else {
        (0 as f64)
    }) * (if ((-(mem[22])) + (mem[4])) >= (0 as f64) {
        (1 as f64)
    } else {
        (0 as f64)
    })) * (if ((((mem[22])
        + ((mem[23])
            * (((-0.5 as f64) + (((-(mem[22])) + (mem[4])) / (mem[23])))
                + ((0.3183098861837907 as f64)
                    * (f64::atan(
                        (1.0 / f64::tan(
                            ((3.141592653589793 as f64) * ((-(mem[22])) + (mem[4]))) / (mem[23]),
                        )),
                    ))))))
        + (mem[25]))
        - (mem[4]))
        >= (0 as f64)
    {
        (1 as f64)
    } else {
        (0 as f64)
    })) > (0.5 as f64)
    {
        mem[28]
    } else {
        (0 as f64)
    });
    let t_66 = (((((f64::powf(t_34, (3 as f64))) * (t_33)) * (t_32)) * (mem[30])) + (mem[27]))
        * ((t_31) - (mem[29]));
    let t_70 = (((t_44) * (mem[26])) * (t_43)) * ((t_35) - (t_36));
    let t_71 = (((0.008 as f64)
        * ((-1 as f64) + (f64::exp((0.04 as f64) * ((77 as f64) + (t_45))))))
        * (t_46))
        / (f64::exp((0.04 as f64) * ((35 as f64) + (t_45))));
    let t_80 = (0.0035 as f64)
        * ((((0.2 as f64) * ((23 as f64) + (t_47)))
            / ((1 as f64) - (f64::exp((-0.04 as f64) * ((23 as f64) + (t_47))))))
            + (((4 as f64) * ((-1 as f64) + (f64::exp((0.04 as f64) * ((85 as f64) + (t_47))))))
                / ((f64::exp((0.04 as f64) * ((53 as f64) + (t_47))))
                    + (f64::exp((0.08 as f64) * ((53 as f64) + (t_47)))))));
    let t_97 = ((-47 as f64) - (t_48))
        / ((-1 as f64) + (f64::exp((-0.1 as f64) * ((47 as f64) + (t_48)))));
    let t_101 = (40 as f64) * (f64::exp((-0.056 as f64) * ((72 as f64) + (t_48))));
    let t_105 = (0.126 as f64) * (f64::exp((-0.25 as f64) * ((77 as f64) + (t_49))));
    let t_109 =
        (1.7 as f64) / ((1 as f64) + (f64::exp((-0.082 as f64) * ((22.5 as f64) + (t_49)))));
    let t_113 = ((0.055 as f64) * (f64::exp((-0.25 as f64) * ((78 as f64) + (t_50)))))
        / ((1 as f64) + (f64::exp((-0.2 as f64) * ((78 as f64) + (t_50)))));
    let t_121 = (0.3 as f64) / ((1 as f64) + (f64::exp((-0.1 as f64) * ((32 as f64) + (t_50)))));
    let t_125 = ((0.0005 as f64)
        * (f64::exp((0.08264462809917356 as f64) * ((50 as f64) + (t_51)))))
        / ((1 as f64) + (f64::exp((0.05714285714285714 as f64) * ((50 as f64) + (t_51)))));
    let t_133 = ((0.0013 as f64)
        * (f64::exp((0.05998800239952009 as f64) * ((-20 as f64) - (t_51)))))
        / ((1 as f64) + (f64::exp((0.04 as f64) * ((-20 as f64) - (t_51)))));
    let t_141 = ((0.095 as f64) * (f64::exp((0.01 as f64) * ((5 as f64) - (t_52)))))
        / ((1 as f64) + (f64::exp((0.07199424046076314 as f64) * ((5 as f64) - (t_52)))));
    let t_148 = ((0.07 as f64)
        * (f64::exp((0.01694915254237288 as f64) * ((-44 as f64) - (t_52)))))
        / ((1 as f64) + (f64::exp((0.05 as f64) * ((44 as f64) + (t_52)))));
    let t_155 = ((0.012 as f64) * (f64::exp((0.008 as f64) * ((-28 as f64) - (t_53)))))
        / ((1 as f64) + (f64::exp((0.14992503748125938 as f64) * ((28 as f64) + (t_53)))));
    let t_162 = ((0.0065 as f64) * (f64::exp((0.02 as f64) * ((-30 as f64) - (t_53)))))
        / ((1 as f64) + (f64::exp((0.2 as f64) * ((-30 as f64) - (t_53)))));
    let t_169 = t_54;
    let t_170 = t_66;
    let t_171 = t_70;
    let t_172 = t_71;
    let t_173 = t_80;
    mem[13] = (((1 as f64) - (mem[5])) * (t_141)) + (((-1 as f64) * (t_148)) * (mem[5]));
    mem[14] = (((-1 as f64) * (t_162)) * (mem[6])) + ((t_155) * ((1 as f64) - (mem[6])));
    mem[15] = ((0.07 as f64) * ((0.0001 as f64) - (mem[7]))) + ((-0.01 as f64) * (t_70));
    mem[16] = (((1 as f64) - (mem[8])) * (t_125)) + (((-1 as f64) * (mem[8])) * (t_133));
    mem[17] = (((-1 as f64) * (t_101)) * (mem[9])) + (((1 as f64) - (mem[9])) * (t_97));
    mem[18] = (((-1 as f64) * (mem[10])) * (t_109)) + (((1 as f64) - (mem[10])) * (t_105));
    mem[19] = (((((-(t_172)) - (t_171)) - (t_173)) - (t_170)) + (t_169)) / (mem[24]);
    mem[20] = (((-1 as f64) * (t_121)) * (mem[12])) + ((t_113) * ((1 as f64) - (mem[12])));
}
