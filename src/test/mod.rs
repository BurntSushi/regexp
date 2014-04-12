#[cfg(bench)]
mod bench;
#[cfg(not(bench))]
mod macro;
#[cfg(not(bench))]
mod tests;

