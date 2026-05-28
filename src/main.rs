mod models;
mod facilities;
mod network;

use crate::facilities::{Station};
use crate::models::{Cargo, Engine, EngineType, Location, StationCommand, STATION_HEARTBEAT_MS, StationTx, StationRx};
use crate::network::{RailwayNetwork, TelemetryLedger};

use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use tokio::sync::mpsc::unbounded_channel;

use tokio::sync::oneshot::{self as tokio_oneshot};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn init_tracing() -> WorkerGuard {
    let file_appender = rolling::daily("target", "hello_thomas.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_filter(EnvFilter::new("hello_thomas=trace")),
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_filter(EnvFilter::new("hello_thomas=info")),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");

    guard
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub stations: Vec<StationConfig>,
    pub tracks: Vec<TrackConfig>,
}

#[derive(Deserialize, Debug)]
pub struct StationConfig {
    pub id: u32,
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize, Debug)]
pub struct TrackConfig {
    pub origin: u32,
    pub destination: u32,
}


#[tokio::main] async fn main() {
    let _log_guard = init_tracing();
    info!("Starting station-centric Sodor simulation...");

    let file_content = std::fs::read_to_string("sodor.json").expect("Failed to read sodor.json");
    let config: Config = serde_json::from_str(&file_content).expect("Failed to parse JSON");

    info!(
        "Loaded {} stations and {} tracks from config.",
        config.stations.len(),
        config.tracks.len()
    );

    let mut network = RailwayNetwork::new();
    let mut switchboard: HashMap<u32, StationTx> = HashMap::new();
    let mut receivers: HashMap<u32, StationRx> = HashMap::new();

    for station in &config.stations {
        //let (tx, rx) = mpsc::channel();
        let (tx, rx) = unbounded_channel::<StationCommand>();
        switchboard.insert(station.id, tx);
        receivers.insert(station.id, rx);
        network.register_station(
            station.id,
            Location {
                x: station.x,
                y: station.y,
            },
        );
    }

    for track in &config.tracks {
        network.add_track(track.origin, track.destination);
    }

    let shared_network = Arc::new(network);
    let telemetry = TelemetryLedger::start();
    let telemetry_client = telemetry.client();
    let sim_tick = Arc::new(AtomicU64::new(0));
    let sim_clock_running = Arc::new(AtomicBool::new(true));

    let sim_tick_clock = Arc::clone(&sim_tick);
    let sim_clock_running_thread = Arc::clone(&sim_clock_running);

    let sim_clock = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(STATION_HEARTBEAT_MS));
        while sim_clock_running_thread.load(Ordering::SeqCst) {
            tick.tick().await;
            sim_tick_clock.fetch_add(1, Ordering::SeqCst);
        }
    });

    let build_neighbors = |
        station_id: u32,
        net: &RailwayNetwork,
        board: &HashMap<u32, StationTx>,
    | -> HashMap<u32, StationTx> {
        net.get_tracks(&station_id)
            .into_iter()
            .flatten()
            .map(|(dest_id, _)| {
                let tx = board
                    .get(dest_id)
                    .expect("Missing neighbor sender")
                    .clone();
                (*dest_id, tx)
            })
            .collect()
    };

    for station in &config.stations {
        let neighbors = build_neighbors(station.id, &shared_network, &switchboard);
        let tx = switchboard
            .get(&station.id)
            .expect("Missing station sender")
            .clone();
        let rx = receivers
            .remove(&station.id)
            .expect("Missing station receiver");

        Station::new(
            station.id,
            &station.name,
            neighbors,
            tx,
            Arc::clone(&shared_network),
            telemetry_client.clone(),
            Arc::clone(&sim_tick),
            rx,
        );
    }

    info!("Stations online. Seeding initial engines and cargo...");

    let mut rng = rand::thread_rng();
    let item_types = [
        "Foam",
        "Bananas",
        "Steel Girders",
        "Welsh Coal",
        "Mail Bags",
        "Passengers",
        "Livestock",
        "Scrap Metal",
        "Electronics",
    ];

    let station_ids: Vec<u32> = config.stations.iter().map(|s| s.id).collect();
    let mut global_engine_id: u32 = 1;
    let cargo_counter = Arc::new(AtomicU32::new(1));

    for station in &config.stations {
        let tx = switchboard
            .get(&station.id)
            .expect("Missing station sender during seed")
            .clone();

        let engine_loadout = vec![
            (EngineType::Thomas, EngineType::Thomas.max_fuel_capacity()),
            (EngineType::Thomas, EngineType::Thomas.max_fuel_capacity()),
            (EngineType::Percy, EngineType::Percy.max_fuel_capacity()),
            (EngineType::Percy, EngineType::Percy.max_fuel_capacity()),
            (EngineType::Diesel, EngineType::Diesel.max_fuel_capacity()),
            (EngineType::Diesel, EngineType::Diesel.max_fuel_capacity()),
            (EngineType::Gordon, EngineType::Gordon.max_fuel_capacity()),
            (EngineType::Gordon, EngineType::Gordon.max_fuel_capacity()),
        ];

        for (engine_type, fuel) in engine_loadout {
            let engine = Engine {
                id: global_engine_id,
                engine_type,
                current_fuel: fuel,
            };
            global_engine_id += 1;

            let (reply_tx, reply_rx) = tokio_oneshot::channel();
            tx.send(StationCommand::IntakeEngine {
                engine,
                reply_to: Some(reply_tx),
            })
            .expect("Failed to send IntakeEngine");

            //let _ = reply_rx.try_recv();
            match reply_rx.await {
                Ok(Ok(())) => {
                    // Successfully added engine
                }
                Ok(Err(e)) => {
                    panic!("Failed to intake engine at station {}: format!{:?}! This shouldn't happen at seed!", station.id, e);
                }
                Err(_) => {
                    panic!("IntakeEngine response channel closed for station {}! But this shouldn't happen at seed!", station.id);
                }
            }
        }

        let mut seed_cargo = Vec::new();
        for _ in 0..14 {
            let mut destination = station.id;
            while destination == station.id {
                destination = station_ids[rng.gen_range(0..station_ids.len())];
            }

            let cargo_id = cargo_counter.fetch_add(1, Ordering::SeqCst);
            let created_time_ms = sim_tick.load(Ordering::SeqCst);
            let ttl_ticks = rng.gen_range(10..=22);
            let expiry_time_ms = created_time_ms.saturating_add(ttl_ticks as u64);

            seed_cargo.push(Cargo {
                id: cargo_id,
                item: item_types[rng.gen_range(0..item_types.len())].to_string(),
                destination,
                actual_weight: rng.gen_range(100..=4000),
                contraband: if rng.gen_ratio(1, 20) {
                    Some("Mystery Box".to_string())
                } else {
                    None
                },
                created_time_ms,
                expiry_time_ms,
                in_transit_since_ms: None,
            });
        }

        let (reply_tx, reply_rx) = tokio_oneshot::channel();
        tx.send(StationCommand::IntakeCargo {
            cargo: seed_cargo,
            reply_to: reply_tx,
        })
        .expect("Failed to seed cargo");

        //let _ = reply_rx.try_recv();
        match reply_rx.await {
            Ok(Ok(())) => {
                // Successfully added cargo
            }
            Ok(Err(e)) => {
                panic!("Failed to intake seed cargo at station {}: format!{:?}! This shouldn't happen at seed!", station.id, e);
            }
            Err(_) => {
                panic!("IntakeCargo response channel closed for station {}! But this shouldn't happen at seed!", station.id);
            }
        }
    }

    info!("Initial world seeded. Starting random cargo generator...");

    let running = Arc::new(AtomicBool::new(true));
    let running_gen = Arc::clone(&running);
    let switchboard_gen = switchboard.clone();
    let station_ids_gen = station_ids.clone();
    let cargo_counter_gen = Arc::clone(&cargo_counter);
    let sim_tick_gen = Arc::clone(&sim_tick);

    let generator = tokio::spawn(async move {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::from_entropy();
        let item_types = [
            "Foam",
            "Bananas",
            "Steel Girders",
            "Welsh Coal",
            "Mail Bags",
            "Passengers",
            "Livestock",
            "Scrap Metal",
            "Electronics",
        ];

        let mut tick = tokio::time::interval(Duration::from_millis(150));
        
        while running_gen.load(Ordering::SeqCst) {

            tick.tick().await;

            //let mut rng = rand::thread_rng();


            let origin = station_ids_gen[rng.gen_range(0..station_ids_gen.len())];
            let candidates: Vec<u32> = station_ids_gen.iter().copied().filter(|&id| id != origin).collect();
            let destination = candidates[rng.gen_range(0..candidates.len())];

            let cargo_id = cargo_counter_gen.fetch_add(1, Ordering::SeqCst);
            let created_time_ms = sim_tick_gen.load(Ordering::SeqCst);
            let ttl_ticks = rng.gen_range(8..=18);
            let expiry_time_ms = created_time_ms.saturating_add(ttl_ticks as u64);

            let cargo = Cargo {
                id: cargo_id,
                item: item_types[rng.gen_range(0..item_types.len())].to_string(),
                destination,
                actual_weight: rng.gen_range(100..=3500),
                contraband: if rng.gen_ratio(1, 30) {
                    Some("Mystery Box".to_string())
                } else {
                    None
                },
                created_time_ms,
                expiry_time_ms,
                in_transit_since_ms: None,
            };

            if let Some(tx) = switchboard_gen.get(&origin) {
                let (reply_tx, reply_rx) = tokio_oneshot::channel();
                // let _ = tx.send(StationCommand::IntakeCargo {
                //     cargo: vec![cargo],
                //     reply_to: reply_tx,
                // });
                match tx.send(StationCommand::IntakeCargo {
                    cargo: vec![cargo],
                    reply_to: reply_tx,
                }) {
                    Ok(()) => {
                        // Successfully sent new cargo
                    }
                    Err(e) => {
                        warn!("Failed to send new cargo to station {}: {}. This might happen if the station is busy or shutting down.", origin, e);
                        continue;
                    }
                }

                
                match tokio::time::timeout(Duration::from_millis(150), reply_rx).await {
                    Ok(Ok(Ok(()))) => {
                        // Successfully added cargo
                    }
                    Ok(Ok(Err(e))) => {
                        warn!("Failed to intake generated cargo at station {}: {:?}! This might happen if the station is busy or shutting down.", origin, e);
                    }
                    Ok(Err(_)) => {
                        warn!("IntakeCargo response channel closed for station {}! This might happen if the station is busy or shutting down.", origin);
                    }
                    Err(_) => {
                        warn!("IntakeCargo response timed out for station {}! This might happen if the station is busy or shutting down.", origin);
                    }
                }
            }
        }
    });

    let simulation_runtime = Duration::from_secs(12);
    info!("Simulation running for {:?}...", simulation_runtime);
    tokio::time::sleep(simulation_runtime).await;

    running.store(false, Ordering::SeqCst);
    let _ = generator.await;

    info!("Generator stopped. Waiting for cargo lifecycle to drain...");
    let drain_deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let Some(snapshot) = telemetry.snapshot() else {
            warn!("Telemetry snapshot unavailable; ending drain wait early.");
            break;
        };

        if snapshot.is_drained() {
            info!(
                "Cargo drained: registered={}, terminal={}, active={}",
                snapshot.cargo_registered,
                snapshot.cargo_terminal,
                snapshot.active_cargo()
            );
            break;
        }

        if Instant::now() >= drain_deadline {
            warn!(
                "Timed out waiting for drain: registered={}, terminal={}, active={}",
                snapshot.cargo_registered,
                snapshot.cargo_terminal,
                snapshot.active_cargo()
            );
            break;
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    for station_id in &station_ids {
        if let Some(tx) = switchboard.get(station_id) {
            let _ = tx.send(StationCommand::PrintStatus);
            let _ = tx.send(StationCommand::Terminate);
        }
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    sim_clock_running.store(false, Ordering::SeqCst);
    let _ = sim_clock.await;

    info!("TelemetryLedger Status: {:#?}", telemetry.snapshot());
    telemetry.shutdown();
    info!("Simulation complete.");
}
