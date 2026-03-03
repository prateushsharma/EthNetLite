fn main() 
{
    tonic_build::compile_protos("proto/ethscope.proto")
    .expect("failed to compile proto/ethscope.proto");
}