using DifferentialEquations
using BenchmarkTools
using Sundials

include("compiler.jl")

ml = CellModel("../models/ohara_rudy_cipa_v1_2017.cellml.xml")
# ml = CellModel("../models/tentusscher_noble_noble_panfilov_2004_a.cellml.xml")
p = get_p(ml)
u0 = get_u0(ml)

function trial(prob)    
    return solve(prob, CVODE_BDF(), dtmax=0.1)
end

prob = ODEProblem(ml.sys, u0, (0, 5000.0), p)
S = trial(prob)
n = length(S)

println("LLVM")
b_llvm = @benchmark trial(prob) setup=(prob=ODEProblem(ml.sys, S[rand(1:n)], (0, 5000.0), p))

println("native")
b_native = @benchmark trial(prob) setup=(prob=ODEProblem(compile(ml.sys, "native"), S[rand(1:n)], (0, 5000.0), p))

println("wasm")
b_wasm = @benchmark trial(prob) setup=(prob=ODEProblem(compile(ml.sys, "wasm"), S[rand(1:n)], (0, 5000.0), p))

println("bytecode")
b_bytecode = @benchmark trial(prob) setup=(prob=ODEProblem(compile(ml.sys, "bytecode"), S[rand(1:n)], (0, 5000.0), p))


