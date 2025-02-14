#[derive(Default, Debug, Clone, PartialEq)]
pub struct Chunk {
    pub offset: usize, // TBD: use tuple (index, Chunk) instead?
    pub start: usize,
    pub end: usize,
    pub text: String,
}

// TODO: conversion impls
// ChunkerTokenizationStreamResult
