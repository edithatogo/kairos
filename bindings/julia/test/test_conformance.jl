using JSON3
using Test

# Fixture metadata only; no Rust runtime execution is claimed.
@testset "conformance fixture metadata" begin
    deterministic = JSON3.read(read("conformance/fixtures/deterministic_ordering.json", String))
    @test deterministic.version == 1
    @test deterministic.expected_kind_order == [1, 2, 4, 3]

    cancellation = JSON3.read(read("conformance/fixtures/cancellation.json", String))
    @test cancellation.expected_kind_order == [1, 3]

    rng_replay = JSON3.read(read("conformance/fixtures/rng_replay.json", String))
    @test rng_replay.run_seed == 7
    @test length(rng_replay.expected_stream) == 4
end
