//! # Pulsar Engine Module
//! 
//! This module contains the core game engine for Pulsar Engine.
//! It manages the main application state and integrates with the UI components.
//! It provides the necessary functionality for running the game loop, handling events,
//! and updating the game state.


use crate::PulsarApp;

pub struct PulsarEngine {
    pub app: PulsarApp,
}

impl PulsarEngine {
    pub fn new(app: PulsarApp) -> Self {
        Self { app }
    }

    pub fn run(&mut self) {
        // Begin spawning the engine threads to handle different tasks

        // Game Thread
        tokio::spawn({
            async move {

            }
        });

        // Render Thread
        tokio::spawn({
            async move {

            }
        });

        // Input Thread
        tokio::spawn({
            async move {

            }
        });

        // Audio Thread
        tokio::spawn({
            async move {

            }
        });

        // Network Thread
        tokio::spawn({
            async move {

            }
        });

        // Physics Thread
        tokio::spawn({
            async move {

            }
        });

        // 
    }
}