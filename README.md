```julia
julia> include("compiler.jl")
test (generic function with 1 method)

julia> using CellMLToolkit, DifferentialEquations, BenchmarkTools

julia> ml = CellModel("/home/shahriar/af/Julia/CellMLToolkit.jl/models/beeler_reuter_1977.cellml.xml");

julia> p = get_p(ml);

julia> u0 = get_u0(ml);

julia> prob = ODEProblem(ml.sys, u0, (0, 5000.0), p);

julia> @benchmark sol = solve(prob, Euler(), dt=0.02)
BenchmarkTools.Trial: 19 samples with 1 evaluation.
 Range (min … max):  138.930 ms … 717.277 ms  ┊ GC (min … max):  0.00% … 80.09%
 Time  (median):     208.293 ms               ┊ GC (median):    26.22%
 Time  (mean ± σ):   263.767 ms ± 171.915 ms  ┊ GC (mean ± σ):  41.79% ± 27.01%

  ▁█▁                                                         ▁  
  ███▆▁▁▁▆▆▁▆▆▁▁▁▁▆▁▆▆▆▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█ ▁
  139 ms           Histogram: frequency by time          717 ms <

 Memory estimate: 116.35 MiB, allocs estimate: 2000056.

julia> f = compile(ml.sys, "native");
number states = 8, number params = 10

julia> prob = ODEProblem(f, u0, (0, 5000.0), p);

julia> @benchmark sol = solve(prob, Euler(), dt=0.02)
BenchmarkTools.Trial: 22 samples with 1 evaluation.
 Range (min … max):  142.301 ms … 914.903 ms  ┊ GC (min … max):  0.00% … 81.80%
 Time  (median):     205.509 ms               ┊ GC (median):    27.10%
 Time  (mean ± σ):   262.886 ms ± 193.052 ms  ┊ GC (mean ± σ):  41.49% ± 26.18%

  ▁█                                                             
  ██▁▄▄▄▄▄▄▇▄▁▄▁▄▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▁▁▁▁▁▁▁▁▁▁▁▁▄ ▁
  142 ms           Histogram: frequency by time          915 ms <

 Memory estimate: 116.35 MiB, allocs estimate: 2000052.

julia> f = compile(ml.sys, "bytecode");
number states = 8, number params = 10

julia> prob = ODEProblem(f, u0, (0, 5000.0), p);

julia> @benchmark sol = solve(prob, Euler(), dt=0.02)
BenchmarkTools.Trial: 12 samples with 1 evaluation.
 Range (min … max):  328.306 ms …    1.007 s  ┊ GC (min … max):  0.00% … 65.10%
 Time  (median):     353.185 ms               ┊ GC (median):     0.00%
 Time  (mean ± σ):   441.562 ms ± 189.815 ms  ┊ GC (mean ± σ):  22.00% ± 20.63%

  ▁█                                                             
  ██▆▁▁▁▁▁▁▁▆▆▆▁▁▁▁▆▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▆ ▁
  328 ms           Histogram: frequency by time          1.01 s <

 Memory estimate: 116.35 MiB, allocs estimate: 2000052.

julia> f = compile(ml.sys, "wasm");
number states = 8, number params = 10

julia> prob = ODEProblem(f, u0, (0, 5000.0), p);

julia> @benchmark sol = solve(prob, Euler(), dt=0.02)
BenchmarkTools.Trial: 4 samples with 1 evaluation.
 Range (min … max):  1.505 s …   1.610 s  ┊ GC (min … max): 0.00% … 7.32%
 Time  (median):     1.510 s              ┊ GC (median):    0.00%
 Time  (mean ± σ):   1.534 s ± 50.817 ms  ┊ GC (mean ± σ):  1.92% ± 3.66%

  █     ▁                                                 ▁  
  █▁▁▁▁▁█▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁█ ▁
  1.5 s          Histogram: frequency by time        1.61 s <

 Memory estimate: 116.35 MiB, allocs estimate: 2000052.
```
