use anyhow::{Context, Result};

#[derive(Debug, Clone, clap::Args)]
pub struct GenerateLinkerArgs {
    #[arg(long, default_value = "0x80000000")]
    pub ram_start: String,

    #[arg(long, default_value = "128Mi")]
    pub ram_size: String,

    #[arg(long, default_value = "64Mi")]
    pub heap_size: String,

    #[arg(long, default_value = "2Mi")]
    pub stack_size: String,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub backtrace: bool,

    #[arg(long, default_value = "_start")]
    pub entry_point: String,
}

#[derive(Debug)]
pub struct LinkerGeneratorResult {
    pub script_content: String,
}

fn parse_address(s: &str) -> Result<usize> {
    if let Some(hex) = s.strip_prefix("0x") {
        usize::from_str_radix(hex, 16).with_context(|| format!("Invalid hex address: {}", s))
    } else {
        s.parse::<usize>()
            .with_context(|| format!("Invalid decimal address: {}", s))
    }
}

pub fn generate_linker_script(args: &GenerateLinkerArgs) -> Result<LinkerGeneratorResult> {
    let ram_start = parse_address(&args.ram_start)?;
    let ram_size = parse_size::parse_size(&args.ram_size)
        .with_context(|| format!("Invalid ram_size: {}", args.ram_size))?
        as usize;
    let heap_size = parse_size::parse_size(&args.heap_size)
        .with_context(|| format!("Invalid heap_size: {}", args.heap_size))?
        as usize;
    let stack_size = parse_size::parse_size(&args.stack_size)
        .with_context(|| format!("Invalid stack_size: {}", args.stack_size))?
        as usize;

    let cfg = crate::linker::LinkerConfig::new()
        .with_memory(ram_start, ram_size)
        .with_heap_size(heap_size)
        .with_stack_size(stack_size)
        .with_backtrace(args.backtrace);

    let mut script_content = cfg.render(Some(
        include_str!("../files/linker.ld.template").to_string(),
    ));

    if args.entry_point != "_start" {
        script_content =
            script_content.replace("ENTRY(_start)", &format!("ENTRY({})", args.entry_point));
    }

    Ok(LinkerGeneratorResult { script_content })
}
