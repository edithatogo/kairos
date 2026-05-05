using Test
using KairoECS

@testset "KairoECS" begin
    @test version_string() == "0.1.0"
    @test self_check() == Dict(
        :package => "KairoECS",
        :version => "0.1.0",
        :status => "ok",
    )
end
