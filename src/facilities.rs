use crate::models::{
    Cargo, Engine, EngineType, Location, MissionReport, RejectedAsset, StationCommand, Train,
    TrainCar, TrainError, STATION_HEARTBEAT_MS,
};
use crate::network::{RailwayNetwork, TelemetryClient, TelemetryEvent};
use rand::Rng;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::{self as tokio_mpsc, UnboundedReceiver, UnboundedSender};
pub type StationTx = tokio_mpsc::UnboundedSender<StationCommand>;
pub type StationRx = tokio_mpsc::UnboundedReceiver<StationCommand>;
// pub type StationTx = Sender<StationCommand>;
// pub type StationRx = Receiver<StationCommand>;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

const EMPTY_CAR_WEIGHT: u32 = 2000;
const ENGINE_REQUEST_MAILBOX_POLL_MS: u64 = 50;

static GLOBAL_CAR_ID: AtomicU32 = AtomicU32::new(0);
static GLOBAL_REQUEST_ID: AtomicU32 = AtomicU32::new(0);
static GLOBAL_TRAIN_ID: AtomicU32 = AtomicU32::new(0);

pub trait CanReport {
    fn get_reporter_name(&self) -> &str;

    fn send_failure_report(
        &self,
        mission_id: u32,
        reason: &str,
        channel: Option<Sender<MissionReport>>,
    ) {
        let name = self.get_reporter_name();
        let message = format!("Mission {} failed at {}. Reason: {}", mission_id, name, reason);
        if let Some(chan) = channel {
            let _ = chan.send(MissionReport::Failure(message));
        } else {
            warn!(
                "[{}] DEAD-LETTER: No reply channel for mission {} failure: {}",
                name, mission_id, reason
            );
        }
    }

    fn send_partial_failure_report(
        &self,
        mission_id: u32,
        reason: &str,
        lost_cargo_ids: &[u32],
        channel: Option<Sender<MissionReport>>,
    ) {
        let name = self.get_reporter_name();
        let message = format!(
            "Mission {} partially failed at {}. Reason: {}. Lost car IDs: {:?}",
            mission_id, name, reason, lost_cargo_ids
        );
        if let Some(chan) = channel {
            let _ = chan.send(MissionReport::PartialFailure(message));
        } else {
            warn!(
                "[{}] DEAD-LETTER: No reply channel for mission {} partial failure",
                name, mission_id
            );
        }
    }

    fn send_success_report(
        &self,
        mission_id: u32,
        details: &str,
        channel: Option<Sender<MissionReport>>,
    ) {
        let name = self.get_reporter_name();
        let message = format!("Mission {} successful at {}. Details: {}", mission_id, name, details);
        if let Some(chan) = channel {
            let _ = chan.send(MissionReport::Success(message));
        } else {
            warn!(
                "[{}] DEAD-LETTER: No reply channel for mission {} success",
                name, mission_id
            );
        }
    }
}

pub struct Railyard {
    pub id: u32,
    pub trains: Vec<Train>,
    pub cars: HashMap<u32, TrainCar>,
    pub purgatory: Vec<RejectedAsset>,
}

impl Railyard {
    fn new(id: u32) -> Self {
        Self {
            id,
            trains: Vec::new(),
            cars: HashMap::new(),
            purgatory: Vec::new(),
        }
    }

    fn generate_new_train_id(&self) -> u32 {
        GLOBAL_TRAIN_ID.fetch_add(1, Ordering::SeqCst)
    }

    fn generate_new_request_id(&self) -> u32 {
        GLOBAL_REQUEST_ID.fetch_add(1, Ordering::SeqCst)
    }

    fn generate_new_car_id(&self) -> u32 {
        GLOBAL_CAR_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub fn validate_empty_car_count(&self, required: usize) -> bool {
        required <= self.cars.values().filter(|car| car.cargo.is_none()).count()
    }

    pub fn load_cargo_into_empty_car(&mut self, cargo: Cargo) -> Result<TrainCar, (Cargo, TrainError)> {
        let empty_car_id = self
            .cars
            .iter()
            .find(|(_, car)| car.cargo.is_none())
            .map(|(&id, _)| id);

        if let Some(id) = empty_car_id {
            let mut car = self.cars.remove(&id).expect("car id must exist");
            car.cargo = Some(cargo);
            Ok(car)
        } else {
            Err((
                cargo,
                TrainError::MissionImpossible {
                    reason: "No empty cars available in yard".to_string(),
                },
            ))
        }
    }

    pub fn assemble_cars(&mut self, cargo: Vec<Cargo>) -> Result<Vec<TrainCar>, (Vec<Cargo>, TrainError)> {
        let mut cars = Vec::with_capacity(cargo.len());
        let mut remaining = cargo;

        while let Some(item) = remaining.pop() { // We pop from the end of the list to preserve ownership without cloning, since we won't need the original list if we successfully load everything.
            match self.load_cargo_into_empty_car(item) {
                Ok(car) => cars.push(car), // Successfully loaded this cargo, move on to the next one.
                Err((returned_cargo, err)) => {
                    // Roll back already loaded cars to preserve ownership without cloning.
                    for mut car in cars.drain(..) {
                        if let Some(loaded_cargo) = car.cargo.take() {
                            remaining.push(loaded_cargo); // Put the cargo back into the remaining list for retrying, since we won't be able to load it onto a train without its car.
                        }
                        self.cars.insert(car.id, car); // Put the empty car back into the yard's inventory
                    }

                    remaining.push(returned_cargo); // Put the cargo that failed to load back into the remaining list for retrying.
                    return Err((remaining, err)); // Return the list of cargo that couldn't be loaded, along with the error from the first failure.
                }
            }
        }

        cars.reverse(); // This is unnecessary but it makes the order of cars more intuitive (first cargo in the list ends up as the first car).
        Ok(cars) // All cargo successfully loaded into cars, return the list of loaded cars.
    }

    pub fn receive_car(&mut self, mut car: TrainCar) -> Result<Option<Cargo>, (TrainCar, Vec<TrainError>)> {
        let mut issues = Vec::new();

        if self.cars.contains_key(&car.id)
            || self.purgatory.iter().any(|asset| asset.car.id == car.id)
        {
            issues.push(TrainError::DuplicateId(car.id));
        }

        if let Some(cargo) = &mut car.cargo {
            if let Err(e) = cargo.check_and_confiscate() {
                issues.push(e);
            }
        }

        if issues.is_empty() {
            let cargo = car.cargo.take();
            self.cars.insert(car.id, car);
            Ok(cargo)
        } else {
            Err((car, issues))
        }
    }

    pub fn purge_expired_cargo_from_purgatory(&mut self, now_tick: u64) -> Vec<Cargo> {
        let mut expired = Vec::new();
        let mut retained = Vec::new();

        for mut asset in self.purgatory.drain(..) {
            if let Some(cargo) = asset.car.cargo.take() {
                if cargo.expiry_time_ms <= now_tick {
                    expired.push(cargo);
                } else {
                    asset.car.cargo = Some(cargo);
                    retained.push(asset);
                }
            } else {
                retained.push(asset);
            }
        }

        self.purgatory = retained;
        expired
    }

    pub fn print_report(&self, roundhouse: &Roundhouse) {
        println!("{BOLD}{CYAN}--- Yard Report [{}] ---{RESET}", self.id);
        println!("Cars in yard: {}", self.cars.len());
        println!("Cars in purgatory: {}", self.purgatory.len());
        let standby: usize = roundhouse.stalls.values().map(|q| q.len()).sum();
        println!("Engines on standby: {}", standby);
    }
}

pub struct Roundhouse {
    pub id: u32,
    pub stalls: HashMap<EngineType, VecDeque<Engine>>,
}

impl Roundhouse {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            stalls: HashMap::new(),
        }
    }

    pub fn house(&mut self, engine: Engine) {
        self.stalls
            .entry(engine.engine_type)
            .or_insert_with(VecDeque::new)
            .push_back(engine);
    }

    pub fn total_engines(&self) -> usize {
        self.stalls.values().map(|q| q.len()).sum()
    }

    pub fn select_engine_by_id(&mut self, engine_id: u32) -> Option<Engine> {
        for queue in self.stalls.values_mut() {
            if let Some(pos) = queue.iter().position(|e| e.id == engine_id) {
                return queue.remove(pos);
            }
        }
        None
    }

    pub fn find_can_fulfill_request(
        &self,
        max_hop_to_requester: f64,
        total_weight: u32,
        max_hop_to_destination: f64,
    ) -> Option<u32> {
        let roster = [
            EngineType::Percy,
            EngineType::Thomas,
            EngineType::Diesel,
            EngineType::Gordon,
        ];

        for etype in roster {
            if let Some(queue) = self.stalls.get(&etype) {
                for engine in queue {
                    if engine.can_complete_mission(0, max_hop_to_requester)
                        && engine.can_complete_mission(total_weight, max_hop_to_destination)
                    {
                        return Some(engine.id);
                    }
                }
            }
        }

        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoQueueItem {
    pub cargo_id: u32,
    pub progress_ppm: u64,
    pub expiry_time_ms: u64,
}

impl CargoQueueItem {
    fn from_cargo(cargo: &Cargo, now_tick: u64) -> Self {
        Self {
            cargo_id: cargo.id,
            progress_ppm: cargo.dispatch_priority_ppm(now_tick),
            expiry_time_ms: cargo.expiry_time_ms,
        }
    }
}

impl Ord for CargoQueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.progress_ppm
            .cmp(&other.progress_ppm)
            .then_with(|| other.expiry_time_ms.cmp(&self.expiry_time_ms))
            .then_with(|| self.cargo_id.cmp(&other.cargo_id))
    }
}

impl PartialOrd for CargoQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Warehouse {
    pub id: u32,
    pub inventory: HashMap<u32, Cargo>,
    pub pending_work: BinaryHeap<CargoQueueItem>,
}

impl Warehouse {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            inventory: HashMap::new(),
            pending_work: BinaryHeap::new(),
        }
    }

    pub fn store(&mut self, mut cargo: Cargo, now_tick: u64) {
        if let Some(in_transit_since_ms) = cargo.in_transit_since_ms.take() {
            let transit_duration_ms = now_tick.saturating_sub(in_transit_since_ms);
            cargo.expiry_time_ms = cargo.expiry_time_ms.saturating_add(transit_duration_ms);
        }

        let queue_item = CargoQueueItem::from_cargo(&cargo, now_tick);
        self.inventory.insert(cargo.id, cargo);
        self.pending_work.push(queue_item);
    }

    pub fn rebuild_pending_work(&mut self, now_tick: u64) {
        let mut rebuilt = BinaryHeap::new();

        for cargo in self.inventory.values() {
            if !cargo.is_expired(now_tick) {
                rebuilt.push(CargoQueueItem::from_cargo(cargo, now_tick));
            }
        }

        self.pending_work = rebuilt;
    }

    pub fn drop_expired_cargo(&mut self, now_tick: u64) -> Vec<u32> {
        let mut expired = Vec::new();

        self.inventory.retain(|id, cargo| {
            let keep = !cargo.is_expired(now_tick);
            if !keep {
                expired.push(*id);
            }
            keep
        });

        self.rebuild_pending_work(now_tick);
        expired
    }

    pub fn pop_next_cargo_for_dispatch(&mut self, now_tick: u64) -> Option<Cargo> {

        while let Some(candidate) = self.pending_work.pop() {
            let Some(current) = self.inventory.get(&candidate.cargo_id) else {
                continue;
            };

            if current.is_expired(now_tick) {
                self.inventory.remove(&candidate.cargo_id);
                continue;
            }

            let live_progress = current.dispatch_priority_ppm(now_tick);
            if live_progress != candidate.progress_ppm || current.expiry_time_ms != candidate.expiry_time_ms {
                self.pending_work.push(CargoQueueItem::from_cargo(current, now_tick));
                continue;
            }

            let mut cargo = self.inventory.remove(&candidate.cargo_id)?;
            cargo.in_transit_since_ms = Some(now_tick);
            return Some(cargo);
        }

        None
    }

    pub fn get_cargo_by_id(&mut self, id: u32, now_tick: u64) -> Result<Cargo, TrainError> {
        match self.inventory.remove(&id) {
            Some(mut cargo) => {
                cargo.in_transit_since_ms = Some(now_tick);
                Ok(cargo)
            }
            None => Err(TrainError::MissingCargo { cargo_id: vec![id] }),
        }
    }
}

pub struct StationMetadata {
    pub id: u32,
    pub name: String,
    pub location: Location,
}

pub struct Station;

impl Station {
    pub fn new(
        id: u32,
        name: &str,
        neighbors: HashMap<u32, StationTx>,
        tx: StationTx,
        map: Arc<RailwayNetwork>,
        telemetry: TelemetryClient,
        sim_tick: Arc<AtomicU64>,
        rx: StationRx,
    ) {
        let station_name = String::from(name);
        let station_id = id;

        let mut state = StationState::new(
            id,
            station_name.clone(),
            neighbors,
            map,
            telemetry,
            tx,
            sim_tick,
        );

        thread::spawn(move || {
            println!(
                "{BOLD}{CYAN}[{}]::Station {} online.{RESET}",
                station_name, station_id
            );

            let mut last_heartbeat = Instant::now();

            loop {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(command) => {
                        if !state.handle_command(command) {
                            println!(
                                "{BOLD}{RED}[{}]::Station {} shutting down.{RESET}",
                                station_name, station_id
                            );
                            break;
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(e) => {
                        println!(
                            "{BOLD}{RED}[{}]::Station {} channel error: {:?}.{RESET}",
                            station_name, station_id, e
                        );
                        break;
                    }
                }

                if last_heartbeat.elapsed() >= Duration::from_millis(STATION_HEARTBEAT_MS) {
                    state.handle_heartbeat();
                    last_heartbeat = Instant::now();
                }
            }
        });
    }
}

pub struct StationState {
    pub id: u32,
    pub name: String,
    pub yard: Railyard,
    pub roundhouse: Roundhouse,
    pub warehouse: Warehouse,
    pub neighbors: HashMap<u32, StationTx>,
    pub map: Arc<RailwayNetwork>,
    pub telemetry: TelemetryClient,
    pub sim_tick: Arc<AtomicU64>,
    pub seen_engine_request: HashSet<u32>,
    pub pending_engine_requests: HashMap<u32, EngineRequestState>,
    pub tx: StationTx,
}

impl CanReport for StationState {
    fn get_reporter_name(&self) -> &str {
        &self.name
    }
}

struct ProcessCarsOutcome {
    failed_car_ids: Vec<u32>,
    failed_cargo_ids: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
enum EngineRequestState {
    Searching { request_id: u32, retry_after: Instant },
    Promised {
        request_id: u32,
        responder_id: u32,
        engine_id: u32,
    },
}

impl StationState {
    pub fn new(
        id: u32,
        name: String,
        neighbors: HashMap<u32, StationTx>,
        map: Arc<RailwayNetwork>,
        telemetry: TelemetryClient,
        tx: StationTx,
        sim_tick: Arc<AtomicU64>,
    ) -> Self {
        Self {
            id,
            name,
            yard: Railyard::new(id),
            roundhouse: Roundhouse::new(id),
            warehouse: Warehouse::new(id),
            neighbors,
            map,
            telemetry,
            sim_tick,
            seen_engine_request: HashSet::new(),
            pending_engine_requests: HashMap::new(),
            tx,
        }
    }

    fn now_tick(&self) -> u64 {
        self.sim_tick.load(Ordering::SeqCst)
    }

    fn handle_command(&mut self, command: StationCommand) -> bool {
        match command {
            StationCommand::Beat => {
                self.handle_heartbeat();
                true
            }
            StationCommand::ReceiveTrain { train, reply_to } => {
                self.handle_receive_train(train, reply_to);
                true
            }
            StationCommand::HandleEmergencySOS {
                mission_id,
                destination,
                surviving_cars,
                report_to,
            } => {
                self.handle_emergency_sos(mission_id, destination, surviving_cars, report_to);
                true
            }
            StationCommand::IntakeCar { cars, reply_to } => {
                self.handle_intake_cars(cars, Some(reply_to));
                true
            }
            StationCommand::IntakeCargo { cargo, reply_to } => {
                self.handle_intake_cargo(cargo, Some(reply_to));
                true
            }
            StationCommand::IntakeEngine { engine, reply_to } => {
                self.handle_intake_engine(engine, Some(reply_to));
                true
            }
            StationCommand::NewNeighbor { neighbor, neighbor_tx } => {
                self.handle_new_neighbor(neighbor, neighbor_tx);
                true
            }
            StationCommand::EngineRequest {
                requester_id,
                forwarder_id,
                request_id,
                mission_id,
                min_capacity,
                max_hop_to_requester,
                mission_max_hop,
                ttl,
                branch_notified,
                notified_count,
                reply_to,
            } => {
                self.handle_engine_request(
                    requester_id,
                    forwarder_id,
                    request_id,
                    mission_id,
                    min_capacity,
                    max_hop_to_requester,
                    mission_max_hop,
                    ttl,
                    branch_notified,
                    notified_count,
                    reply_to,
                );
                true
            }
            StationCommand::EngineRequestConfirmed {
                request_id,
                mission_id,
                responder_id,
                engine_id,
            } => {
                self.handle_engine_request_confirmed(request_id, mission_id, responder_id, engine_id);
                true
            }
            StationCommand::EngineTransferFailed {
                request_id,
                mission_id,
                responder_id,
                reason,
            } => {
                self.handle_engine_transfer_failed(request_id, mission_id, responder_id, reason);
                true
            }
            StationCommand::CheckStatus => {
                self.try_dispatch_from_warehouse();
                true
            }
            StationCommand::PrintStatus => {
                self.print_status();
                true
            }
            StationCommand::Terminate => false,
        }
    }

    pub fn handle_heartbeat(&mut self) {
        debug!("[{}]::Station {} heartbeat", self.name, self.id);
        let now_tick = self.now_tick();

        let dropped_cargo = self.warehouse.drop_expired_cargo(now_tick);
        if !dropped_cargo.is_empty() {
            for cargo_id in &dropped_cargo {
                self.pending_engine_requests.remove(cargo_id);
            }
            self.telemetry
                .record(TelemetryEvent::CargoExpiredInWarehouse { cargo_ids: dropped_cargo });
        }

        let dropped_from_purgatory = self.yard.purge_expired_cargo_from_purgatory(now_tick);
        if !dropped_from_purgatory.is_empty() {
            let cargo_ids: Vec<u32> = dropped_from_purgatory.into_iter().map(|c| c.id).collect();
            for cargo_id in &cargo_ids {
                self.pending_engine_requests.remove(cargo_id);
            }
            self.telemetry
                .record(TelemetryEvent::CargoExpiredInPurgatory { cargo_ids });
        }

        self.warehouse.rebuild_pending_work(now_tick);
        self.try_dispatch_from_warehouse();
    }

    fn park_cargo_for_retry(&mut self, mut cargo: Cargo) {
        cargo.in_transit_since_ms = None;
        let now_tick = self.now_tick();
        self.warehouse.store(cargo, now_tick);
        self.warehouse.rebuild_pending_work(now_tick);
    }

    fn handle_engine_request_confirmed(
        &mut self,
        request_id: u32,
        mission_id: Option<u32>,
        responder_id: u32,
        engine_id: u32,
    ) {
        let Some(cargo_id) = mission_id else {
            return;
        };

        match self.pending_engine_requests.get(&cargo_id).copied() {
            Some(EngineRequestState::Searching {
                request_id: active_request_id,
                ..
            }) if active_request_id == request_id => {
                info!(
                    "[{}] Request {} for Cargo {} promised by Station {} (Engine {})",
                    self.name, request_id, cargo_id, responder_id, engine_id
                );
                self.pending_engine_requests.insert(
                    cargo_id,
                    EngineRequestState::Promised {
                        request_id,
                        responder_id,
                        engine_id,
                    },
                );
            }
            Some(_) => {
                debug!(
                    "[{}] Ignoring stale EngineRequestConfirmed for Cargo {} (request_id={})",
                    self.name, cargo_id, request_id
                );
            }
            None => {
                debug!(
                    "[{}] Ignoring EngineRequestConfirmed for Cargo {} with no pending request",
                    self.name, cargo_id
                );
            }
        }
    }

    fn handle_engine_transfer_failed(
        &mut self,
        request_id: u32,
        mission_id: Option<u32>,
        responder_id: u32,
        reason: String,
    ) {
        let Some(cargo_id) = mission_id else {
            return;
        };

        match self.pending_engine_requests.get(&cargo_id).copied() {
            Some(EngineRequestState::Promised {
                request_id: active_request_id,
                responder_id: promised_responder_id,
                ..
            }) if active_request_id == request_id && promised_responder_id == responder_id => {
                warn!(
                    "[{}] Engine transfer failed for Cargo {} on request {} from Station {}: {}",
                    self.name, cargo_id, request_id, responder_id, reason
                );
                self.pending_engine_requests.remove(&cargo_id);
                self.try_dispatch_from_warehouse();
            }
            Some(EngineRequestState::Promised {
                request_id: active_request_id,
                responder_id: promised_responder_id,
                ..
            }) if active_request_id == request_id => { // Because we've made it this far, we know the responder_id doesn't match the promised one
                debug!(
                    "[{}] Ignoring transfer failure for Cargo {} on request {} from non-promised responder {} (promised responder is {})",
                    self.name, cargo_id, request_id, responder_id, promised_responder_id
                );
            }
            Some(EngineRequestState::Searching {
                request_id: active_request_id,
                ..
            }) if active_request_id == request_id => {
                warn!(
                    "[{}] ***THIS SHOULDN'T HAPPEN***: Ignoring transfer failure for Cargo {} on request {} while request is still Searching",
                    self.name, cargo_id, request_id
                );
            }
            Some(_) => {
                debug!(
                    "[{}] Ignoring stale transfer failure for Cargo {} (request_id={}, responder_id={})",
                    self.name, cargo_id, request_id, responder_id
                );
            }
            None => {
                debug!(
                    "[{}] Ignoring transfer failure for Cargo {} with no pending engine request",
                    self.name, cargo_id
                );
            }
        }
    }

    fn try_dispatch_from_warehouse(&mut self) {
        let now_tick = self.now_tick();
        self.warehouse.rebuild_pending_work(now_tick);

        while let Some(cargo) = self.warehouse.pop_next_cargo_for_dispatch(now_tick) {
            if !self.dispatch_cargo(cargo) {
                break;
            }
        }
    }

    fn dispatch_cargo(&mut self, cargo: Cargo) -> bool {
        let mission_id = cargo.id;
        let destination = cargo.destination;

        if destination == self.id {
            self.pending_engine_requests.remove(&mission_id);
            self.telemetry
                .record(TelemetryEvent::CargoDelivered { cargo_ids: vec![mission_id] });
            trace!(
                "LIFECYCLE [Cargo {}]: already at destination {}",
                mission_id,
                self.id
            );
            return true;
        }

        let (_, route) = match self.map.find_shortest_path(self.id, destination) {
            Some(path) => path,
            None => {
                warn!(
                    "[{}] No route from {} to {} for Cargo {}",
                    self.name, self.id, destination, mission_id
                );
                self.park_cargo_for_retry(cargo);
                return false;
            }
        };

        let true_total_weight = cargo.actual_weight + EMPTY_CAR_WEIGHT;

        let required_cars = 1usize;
        if !self.yard.validate_empty_car_count(required_cars) {
            self.allocate_empty_cars(required_cars as u32);
            if !self.yard.validate_empty_car_count(required_cars) {
                warn!(
                    "[{}] Unable to allocate required empty cars ({}) for Cargo {}",
                    self.name, required_cars, mission_id
                );
                self.park_cargo_for_retry(cargo);
                return false;
            }
        }

        let mut max_hop_distance = 0.0;
        if route.len() > 1 {
            for i in 0..route.len() - 1 {
                if let Some(dist) = self.map.get_distance(route[i], route[i + 1]) {
                    if dist > max_hop_distance {
                        max_hop_distance = dist;
                    }
                }
            }
        }

        let engine = match self
            .roundhouse
            .find_can_fulfill_request(0.0, true_total_weight, max_hop_distance)
        {
            Some(engine_id) => self
                .roundhouse
                .select_engine_by_id(engine_id)
                .expect("engine id must exist"), // Safe because we just found this ID in the roundhouse
            None => {
                let now = Instant::now();
                let should_request = match self.pending_engine_requests.get(&mission_id).copied() {
                    Some(EngineRequestState::Promised {
                        request_id,
                        responder_id,
                        engine_id,
                    }) => {
                        debug!(
                            "[{}] Cargo {} already has promised engine {} from Station {} (request_id={})",
                            self.name, mission_id, engine_id, responder_id, request_id
                        );
                        false
                    }
                    Some(EngineRequestState::Searching { retry_after, .. }) if now < retry_after => {
                        debug!(
                            "[{}] Engine request for Cargo {} still searching; retry in {} ms",
                            self.name,
                            mission_id,
                            retry_after.saturating_duration_since(now).as_millis()
                        );
                        false
                    }
                    _ => true,
                };

                if should_request {
                    let request_id = self.yard.generate_new_request_id();
                    let request_ttl: u32 = 8;
                    let search_timeout_ms =
                        STATION_HEARTBEAT_MS + (request_ttl as u64 * ENGINE_REQUEST_MAILBOX_POLL_MS);
                    warn!(
                        "[{}] No local engine for Cargo {}; requesting aid (request_id={})",
                        self.name, mission_id, request_id
                    );
                    self.initiate_engine_request(
                        self.id,
                        request_id,
                        Some(mission_id),
                        true_total_weight,
                        max_hop_distance,
                        request_ttl,
                    );
                    self.pending_engine_requests.insert(
                        mission_id,
                        EngineRequestState::Searching {
                            request_id,
                            retry_after: now + Duration::from_millis(search_timeout_ms),
                        },
                    );
                }

                self.park_cargo_for_retry(cargo);
                return false;
            }
        };

        let attached_cars = match self.yard.assemble_cars(vec![cargo]) {
            Ok(cars) => cars,
            Err((returned_cargo, e)) => {
                error!(
                    "[{}] Failed to assemble car for Cargo {}: {:?}",
                    self.name, mission_id, e
                );
                self.roundhouse.house(engine);
                for item in returned_cargo {
                    self.park_cargo_for_retry(item);
                }
                return false;
            }
        };

        let train = Train {
            id: self.yard.generate_new_train_id(),
            engine,
            cars: attached_cars,
            mission_id: Some(mission_id),
            request_id: None,
            destination,
            report_to: None,
            engine_request_reply_to: None,
        };

        self.pending_engine_requests.remove(&mission_id);
        self.dispatch_train(train, route, self.telemetry.clone());
        true
    }

    pub fn handle_receive_train(&mut self, mut train: Train, reply_to: Sender<Result<(), TrainError>>) {
        train.engine.refuel();

        if train.request_id.is_some() {
            if let Some(mission_id) = train.mission_id {
                self.pending_engine_requests.remove(&mission_id);
            }
            if !train.cars.is_empty() {
                let _ = self.process_cars(train.cars, train.mission_id, false);
            }
            self.roundhouse.house(train.engine);
            let _ = reply_to.send(Ok(()));
            self.try_dispatch_from_warehouse();
            return;
        }

        let _ = reply_to.send(Ok(()));

        if self.id == train.destination {
            let mission_id = train.mission_id;
            let report_to = train.report_to;

            self.roundhouse.house(train.engine);
            let outcome = self.process_cars(train.cars, mission_id, true);

            if outcome.failed_car_ids.is_empty() {
                if let Some(mid) = mission_id {
                    self.telemetry
                        .record(TelemetryEvent::CargoDelivered { cargo_ids: vec![mid] });
                    self.send_success_report(mid, "Cargo delivered and processed at destination.", report_to);
                }
            } else {
                if let Some(mid) = mission_id {
                    let reason = format!(
                        "Cargo reached destination but intake failed for car IDs {:?} and cargo IDs {:?}; cargo moved to purgatory and remains retryable until expiry.",
                        outcome.failed_car_ids,
                        outcome.failed_cargo_ids
                    );
                    self.send_partial_failure_report(mid, &reason, &outcome.failed_cargo_ids, report_to);
                }
            }

            self.try_dispatch_from_warehouse();
            return;
        }

        let final_destination = train.destination;
        let route = match self.map.find_shortest_path(self.id, final_destination) {
            Some((_, route)) => route,
            None => {
                self.roundhouse.house(train.engine);
                let _ = self.process_cars(train.cars, train.mission_id, false);
                warn!(
                    "[{}] No forward route from {} to {}; cargo re-queued for retry until expiry",
                    self.name, self.id, final_destination
                );
                self.try_dispatch_from_warehouse();
                return;
            }
        };

        self.dispatch_train(train, route, self.telemetry.clone());
    }

    pub fn handle_emergency_sos(
        &mut self,
        mission_id: u32,
        _destination: u32,
        surviving_cars: Vec<TrainCar>,
        report_to: Option<Sender<MissionReport>>,
    ) {
        println!("{RED}[{}] EMERGENCY SOS for Mission {}.{RESET}", self.name, mission_id);

        let outcome = self.process_cars(surviving_cars, Some(mission_id), false);
        let reason = if outcome.failed_car_ids.is_empty() {
            "Engine lost, cargo salvaged and returned to warehouse for re-dispatch.".to_string()
        } else {
            format!(
                "Engine lost and some cars failed salvage intake. Car IDs: {:?}, Cargo IDs: {:?}",
                outcome.failed_car_ids, outcome.failed_cargo_ids
            )
        };

        self.send_partial_failure_report(mission_id, &reason, &outcome.failed_cargo_ids, report_to);

        let now_tick = self.now_tick();
        self.warehouse.rebuild_pending_work(now_tick);
        self.try_dispatch_from_warehouse();
    }

    fn handle_intake_cars(&mut self, cars: Vec<TrainCar>, reply_to: Option<Sender<Result<(), TrainError>>>) {
        let mut intake_issues = Vec::new();
        let mut purgatory_cargo_ids = Vec::new();
        let now_tick = self.now_tick();

        for car in cars {
            match self.yard.receive_car(car) {
                Ok(Some(cargo)) => self.warehouse.store(cargo, now_tick),
                Ok(None) => {}
                Err((homeless_car, e)) => {
                    intake_issues.push(homeless_car.id);
                    if let Some(cargo) = homeless_car.cargo.as_ref() {
                        purgatory_cargo_ids.push(cargo.id);
                    }
                    let rejected_asset = RejectedAsset::new(homeless_car, e, None);
                    self.yard.purgatory.push(rejected_asset);
                }
            }
        }

        if !purgatory_cargo_ids.is_empty() {
            self.telemetry
                .record(TelemetryEvent::CargoSentToPurgatory { cargo_ids: purgatory_cargo_ids });
        }

        self.warehouse.rebuild_pending_work(now_tick);
        self.try_dispatch_from_warehouse();

        if let Some(channel) = reply_to {
            if intake_issues.is_empty() {
                let _ = channel.send(Ok(()));
            } else {
                let _ = channel.send(Err(TrainError::ContrabandOnBoard(format!(
                    "Cars to purgatory: {:?}",
                    intake_issues
                ))));
            }
        }
    }

    fn handle_intake_cargo(&mut self, cargo: Vec<Cargo>, reply_to: Option<Sender<Result<(), TrainError>>>) {
        let cargo_ids: Vec<u32> = cargo.iter().map(|item| item.id).collect();
        if !cargo_ids.is_empty() {
            self.telemetry
                .record(TelemetryEvent::CargoRegistered { cargo_ids });
        }

        let now_tick = self.now_tick();

        for item in cargo {
            self.warehouse.store(item, now_tick);
        }

        self.warehouse.rebuild_pending_work(now_tick);
        self.try_dispatch_from_warehouse();

        if let Some(channel) = reply_to {
            let _ = channel.send(Ok(()));
        }
    }

    pub fn handle_intake_engine(&mut self, engine: Engine, reply_to: Option<Sender<Result<(), TrainError>>>) {
        self.roundhouse.house(engine);
        if let Some(channel) = reply_to {
            let _ = channel.send(Ok(()));
        }
        self.try_dispatch_from_warehouse();
    }

    pub fn handle_new_neighbor(&mut self, neighbor: u32, tx: StationTx) {
        self.neighbors.insert(neighbor, tx);
    }

    pub fn allocate_empty_cars(&mut self, count: u32) {
        for _ in 0..count {
            let safe_id = self.yard.generate_new_car_id();
            let new_car = TrainCar {
                id: safe_id,
                cargo: None,
                passenger: None,
            };

            if let Err((homeless_car, error)) = self.yard.receive_car(new_car) {
                let rejected_asset = RejectedAsset::new(homeless_car, error, None);
                self.yard.purgatory.push(rejected_asset);
            }
        }
    }

    pub fn handle_engine_request(
        &mut self,
        requester_id: u32,
        forwarder_id: u32,
        request_id: u32,
        mission_id: Option<u32>,
        min_capacity: u32,
        mut max_hop_to_requester: f64,
        mut mission_max_hop: f64,
        ttl: u32,
        branch_notified: [u32; 64],
        notified_count: usize,
        reply_to: Sender<StationCommand>,
    ) {
        if ttl == 0 {
            return;
        }

        // TTL is consumed on arrival at every receiving station.
        let remaining_ttl = ttl.saturating_sub(1);

        if self.seen_engine_request.contains(&request_id) {
            return;
        }
        self.seen_engine_request.insert(request_id);

        if let Some((distance, _)) = self.map.find_shortest_path(self.id, forwarder_id) {
            if distance > max_hop_to_requester {
                max_hop_to_requester = distance;
            }
        }

        let route_to_requester = match self.map.find_shortest_path(self.id, requester_id) {
            Some((_, route)) => route,
            None => return,
        };

        if route_to_requester.len() > 1 {
            for i in 0..route_to_requester.len() - 1 {
                if let Some(dist) = self
                    .map
                    .get_distance(route_to_requester[i], route_to_requester[i + 1])
                {
                    if dist > mission_max_hop {
                        mission_max_hop = dist;
                    }
                }
            }
        }

        if self.roundhouse.total_engines() > 1 {
            if let Some(engine_id) = self
                .roundhouse
                .find_can_fulfill_request(max_hop_to_requester, min_capacity, mission_max_hop)
            {
                if let Some(engine) = self.roundhouse.select_engine_by_id(engine_id) {
                    let _ = reply_to.send(StationCommand::EngineRequestConfirmed {
                        request_id,
                        mission_id,
                        responder_id: self.id,
                        engine_id: engine.id,
                    });
                    info!(
                        "[{}] Engine {} answering request {} for requester {}",
                        self.name, engine.id, request_id, requester_id
                    );
                    let transfer = Train {
                        id: self.yard.generate_new_train_id(),
                        engine,
                        cars: Vec::new(),
                        mission_id,
                        request_id: Some(request_id),
                        destination: requester_id,
                        report_to: None,
                        engine_request_reply_to: Some(reply_to.clone()),
                    };
                    self.dispatch_train(transfer, route_to_requester, self.telemetry.clone());
                }
            }
        }


        // // Copilot, let's take the neighbors vec and shuffle it into an array
        // let randomized_neighbors: Vec<u32> = self.neighbors.keys().copied().collect();
        // let mut rng = rand::thread_rng();
        // let mut shuffled_neighbors = randomized_neighbors.clone();
        // shuffled_neighbors.shuffle(&mut rng);





        self.forward_engine_request(
            requester_id,
            request_id,
            mission_id,
            min_capacity,
            max_hop_to_requester,
            mission_max_hop,
            remaining_ttl,
            branch_notified,
            notified_count,
            reply_to,
        );








    }

    fn initiate_engine_request(
        &mut self,
        requester_id: u32,
        request_id: u32,
        mission_id: Option<u32>,
        min_capacity: u32,
        mission_max_hop: f64,
        ttl: u32,
    ) {
        info!(
            "[{}] Initiating engine request {} for cargo {:?} (min_capacity={}, max_hop={:.2}, ttl={})",
            self.name, request_id, mission_id, min_capacity, mission_max_hop, ttl
        );
        let mut branch_notified = [0u32; 64];
        branch_notified[0] = requester_id;
        self.forward_engine_request(
            requester_id,
            request_id,
            mission_id,
            min_capacity,
            0.0,
            mission_max_hop,
            ttl,
            branch_notified,
            1,
            self.tx.clone(),
        );
    }

    fn forward_engine_request(
        &self,
        requester_id: u32,
        request_id: u32,
        mission_id: Option<u32>,
        min_capacity: u32,
        max_hop_to_requester: f64,
        mission_max_hop: f64,
        ttl: u32,
        branch_notified: [u32; 64],
        notified_count: usize,
        reply_to: Sender<StationCommand>,
    ) {
        if ttl == 0 {
            return;
        }

        use rand::seq::SliceRandom;

        let visited = &branch_notified[..notified_count.min(branch_notified.len())]; // Only consider the portion of the array that has been populated with notified station IDs.
        let mut candidates: Vec<u32> = self
            .neighbors
            .keys()
            .copied()
            .filter(|id| !visited.contains(id))
            .collect();

        if candidates.is_empty() {
            return;
        }

        candidates.shuffle(&mut rand::thread_rng());
        let fan_out = usize::min(ttl as usize, candidates.len());
        if fan_out == 0 {
            return;
        }

        let chosen = &candidates[..fan_out];

        // Stamp all intended recipients before sending so siblings can avoid redundant triangles.
        let mut stamped = branch_notified;
        let mut stamped_count = notified_count;
        for &recipient_id in chosen {
            if stamped_count < stamped.len() {
                stamped[stamped_count] = recipient_id;
                stamped_count += 1;
            }
        }

        let base_ttl = ttl / chosen.len() as u32;
        let ttl_remainder = ttl % chosen.len() as u32;

        debug!(
            "[{}] Forwarding request {} to {} neighbors (ttl={}, base_ttl={}, remainder={})",
            self.name, request_id, fan_out, ttl, base_ttl, ttl_remainder
        );

        for (i, &neighbor_id) in chosen.iter().enumerate() { // Assign the remainder TTL to the first few neighbors to ensure full TTL utilization
            let assigned_ttl = if i < ttl_remainder as usize {
                base_ttl + 1
            } else {
                base_ttl
            };

            if assigned_ttl == 0 {
                continue;
            }

            if let Some(neighbor) = self.neighbors.get(&neighbor_id) {
                let _ = neighbor.send(StationCommand::EngineRequest {
                    requester_id,
                    forwarder_id: self.id,
                    request_id,
                    mission_id,
                    min_capacity,
                    max_hop_to_requester,
                    mission_max_hop,
                    ttl: assigned_ttl,
                    branch_notified: stamped,
                    notified_count: stamped_count,
                    reply_to: reply_to.clone(),
                });
            }
        }
    }

    pub fn print_status(&self) {
        println!("{BOLD}{CYAN}--- Station {} ---{RESET}", self.name);
        self.yard.print_report(&self.roundhouse);
        println!("Warehouse cargo: {}", self.warehouse.inventory.len());
    }

    fn process_cars(
        &mut self,
        cars: Vec<TrainCar>,
        mission_id: Option<u32>,
        is_final_destination: bool,
    ) -> ProcessCarsOutcome {
        let mut failed_car_ids = Vec::new();
        let mut failed_cargo_ids = Vec::new();
        let mut purgatory_cargo_ids = Vec::new();
        let now_tick = self.now_tick();

        for car in cars {
            match self.yard.receive_car(car) {
                Ok(Some(cargo)) => {
                    if is_final_destination {
                        trace!(
                            "LIFECYCLE [Cargo {}]: consumed at destination {}",
                            cargo.id, self.id
                        );
                    } else {
                        self.warehouse.store(cargo, now_tick);
                    }
                }
                Ok(None) => {}
                Err((homeless_car, issues)) => {
                    failed_car_ids.push(homeless_car.id);
                    if let Some(cargo) = homeless_car.cargo.as_ref() {
                        failed_cargo_ids.push(cargo.id);
                        purgatory_cargo_ids.push(cargo.id);
                    }
                    self.yard
                        .purgatory
                        .push(RejectedAsset::new(homeless_car, issues, mission_id));
                }
            }
        }

        if !purgatory_cargo_ids.is_empty() {
            self.telemetry
                .record(TelemetryEvent::CargoSentToPurgatory { cargo_ids: purgatory_cargo_ids });
        }

        ProcessCarsOutcome {
            failed_car_ids,
            failed_cargo_ids,
        }
    }

    pub fn dispatch_train(&self, mut train: Train, route: Vec<u32>, telemetry: TelemetryClient) {
        let final_destination = train.destination;
        let station_tx_clone = self.tx.clone();

        let next_stop = route.get(1).copied().unwrap_or(final_destination);
        let Some(next_stop_handle) = self.neighbors.get(&next_stop).cloned() else {
            warn!("[{}] Next stop {} is not a known neighbor", self.name, next_stop);
            return;
        };

        let Some(distance_to_next_stop) = self.map.get_distance(self.id, next_stop) else {
            warn!(
                "[{}] Missing distance {} -> {}",
                self.name, self.id, next_stop
            );
            return;
        };

        let train_id = train.id;
        let station_name_clone = self.name.clone();
        let station_id_clone = self.id;
        let (transit_tx, transit_rx) = mpsc::channel();

        thread::spawn(move || {
            let time_to_travel = match train.dispatch(distance_to_next_stop) {
                Ok(t) => t,
                Err(e) => {
                    error!(
                        "[{}::Station {}] Failed to dispatch train {}: {:?}",
                        station_name_clone, station_id_clone, train_id, e
                    );
                    telemetry.record(TelemetryEvent::TrainDispatchFailed { reason: format!("{:?}", e) });
                    if let Some(request_id) = train.request_id {
                        if let Some(reply_to) = train.engine_request_reply_to.as_ref() {
                            let _ = reply_to.send(StationCommand::EngineTransferFailed {
                                request_id,
                                mission_id: train.mission_id,
                                responder_id: station_id_clone,
                                reason: format!("Failed to dispatch train {}: {:?}", train_id, e),
                            });
                        }
                    }
                    if let Some(mission_id) = train.mission_id {
                        if !train.cars.is_empty() {
                            match station_tx_clone.send(StationCommand::HandleEmergencySOS {
                                mission_id,
                                destination: train.destination,
                                surviving_cars: train.cars,
                                report_to: train.report_to,
                            }) {
                                Ok(_) => {
                                    info!(
                                        "[{}::Station {}] Reported emergency SOS for train {} after dispatch failure",
                                        station_name_clone, station_id_clone, train_id
                                    );
                                }
                                Err(mpsc::SendError(StationCommand::HandleEmergencySOS {
                                    surviving_cars,
                                    ..
                                })) => {
                                    let reason = format!(
                                        "Failed to report emergency SOS for train {} after dispatch failure",
                                        train_id
                                    );
                                    error!(
                                        "[{}::Station {}] {}",
                                        station_name_clone, station_id_clone, reason
                                    );
                                    telemetry.record(TelemetryEvent::EmergencySOSFailed {
                                        reason: reason.clone(),
                                    });
                                    let cargo_ids: Vec<u32> = surviving_cars
                                        .iter()
                                        .filter_map(|car| car.cargo.as_ref().map(|c| c.id))
                                        .collect();
                                    if !cargo_ids.is_empty() {
                                        telemetry.record(TelemetryEvent::CargoExpiredInTransit { cargo_ids });
                                    }
                                }
                                Err(mpsc::SendError(other)) => {
                                    let reason = format!(
                                        "Failed to report emergency SOS for train {} after dispatch failure; unexpected payload: {:?}",
                                        train_id, other
                                    );
                                    error!(
                                        "[{}::Station {}] {}",
                                        station_name_clone, station_id_clone, reason
                                    );
                                    telemetry.record(TelemetryEvent::EmergencySOSFailed { reason });
                                }
                            }
                        }
                    }
                    return;
                }
            };

            info!(
                "[{}::Station {}] Train {} en route to {} ({:.2}s)",
                station_name_clone,
                station_id_clone,
                train_id,
                next_stop,
                time_to_travel
            );

            thread::sleep(Duration::from_secs_f64(time_to_travel));

            let derail = rand::thread_rng().gen_bool(0.1);
            if derail {
                if let Some(request_id) = train.request_id {
                    error!(
                        "DERAILMENT: transfer train {} for request {}",
                        train_id, request_id
                    );
                    if let Some(reply_to) = train.engine_request_reply_to.as_ref() {
                        let _ = reply_to.send(StationCommand::EngineTransferFailed {
                            request_id,
                            mission_id: train.mission_id,
                            responder_id: station_id_clone,
                            reason: format!(
                                "Emergency engine transfer train {} derailed",
                                train_id
                            ),
                        });
                    }
                    return;
                }

                if let Some(mission_id) = train.mission_id {
                    if !train.cars.is_empty() {
                        telemetry.record(TelemetryEvent::TrainDerailed);
                        match station_tx_clone.send(StationCommand::HandleEmergencySOS {
                            mission_id,
                            destination: train.destination,
                            surviving_cars: train.cars,
                            report_to: train.report_to,
                        }) {
                            Ok(_) => {
                                info!(
                                    "[{}::Station {}] Reported emergency SOS for derailed train {}",
                                    station_name_clone, station_id_clone, train_id
                                );
                            }
                            Err(mpsc::SendError(StationCommand::HandleEmergencySOS {
                                surviving_cars,
                                ..
                            })) => {
                                let reason = format!(
                                    "Failed to report emergency SOS for derailed train {}",
                                    train_id
                                );
                                error!(
                                    "[{}::Station {}] {}",
                                    station_name_clone, station_id_clone, reason
                                );
                                telemetry.record(TelemetryEvent::EmergencySOSFailed {
                                    reason: reason.clone(),
                                });
                                let cargo_ids: Vec<u32> = surviving_cars
                                    .iter()
                                    .filter_map(|car| car.cargo.as_ref().map(|c| c.id))
                                    .collect();
                                if !cargo_ids.is_empty() {
                                    telemetry.record(TelemetryEvent::CargoExpiredInTransit {
                                        cargo_ids,
                                    });
                                }
                            }
                            Err(mpsc::SendError(other)) => {
                                let reason = format!(
                                    "Failed to report emergency SOS for derailed train {} ; unexpected payload: {:?}",
                                    train_id, other
                                );
                                error!(
                                    "[{}::Station {}] {}",
                                    station_name_clone, station_id_clone, reason
                                );
                                telemetry.record(TelemetryEvent::EmergencySOSFailed { reason });
                            }
                        }
                    }
                }

                return;
            }
            let transfer_failure_target = train.request_id.and_then(|request_id| {
                train
                    .engine_request_reply_to
                    .as_ref()
                    .map(|reply_to| (request_id, train.mission_id, reply_to.clone()))
            });

            match next_stop_handle.send(StationCommand::ReceiveTrain {
                train,
                reply_to: transit_tx,
            }) {
                Ok(_) => {
                    info!(
                        "[{}::Station {}] Train {} successfully forwarded to {}",
                        station_name_clone, station_id_clone, train_id, next_stop
                    );
                }
                Err(mpsc::SendError(StationCommand::ReceiveTrain { train, .. })) => {
                    error!(
                        "[{}::Station {}] Failed to forward train {} to {}",
                        station_name_clone, station_id_clone, train_id, next_stop
                    );

                    telemetry.record(TelemetryEvent::TrainForwardFailed { reason: format!("Failed to forward train {} to {}", train_id, next_stop) });
                    if let Some(request_id) = train.request_id {
                        if let Some(reply_to) = train.engine_request_reply_to.as_ref() {
                            let _ = reply_to.send(StationCommand::EngineTransferFailed {
                                request_id,
                                mission_id: train.mission_id,
                                responder_id: station_id_clone,
                                reason: format!(
                                    "Failed to forward train {} to {}: destination station unreachable",
                                    train_id, next_stop
                                ),
                            });
                        }
                    }
                    if let Some(mission_id) = train.mission_id {
                        if !train.cars.is_empty() {
                            match station_tx_clone.send(StationCommand::HandleEmergencySOS {
                                mission_id,
                                destination: train.destination,
                                surviving_cars: train.cars,
                                report_to: train.report_to,
                            }) {
                                Ok(_) => {
                                    info!(
                                        "[{}::Station {}] Reported emergency SOS for train {} after forward failure",
                                        station_name_clone, station_id_clone, train_id
                                    );
                                }
                                Err(mpsc::SendError(StationCommand::HandleEmergencySOS {
                                    surviving_cars,
                                    ..
                                })) => {
                                    let reason = format!(
                                        "Failed to report emergency SOS for train {} after forward failure",
                                        train_id
                                    );
                                    error!(
                                        "[{}::Station {}] {}",
                                        station_name_clone, station_id_clone, reason
                                    );
                                    telemetry.record(TelemetryEvent::EmergencySOSFailed {
                                        reason: reason.clone(),
                                    });
                                    let cargo_ids: Vec<u32> = surviving_cars
                                        .iter()
                                        .filter_map(|car| car.cargo.as_ref().map(|c| c.id))
                                        .collect();
                                    if !cargo_ids.is_empty() {
                                        telemetry.record(TelemetryEvent::CargoExpiredInTransit {
                                            cargo_ids,
                                        });
                                    }
                                }
                                Err(mpsc::SendError(other)) => {
                                    let reason = format!(
                                        "Failed to report emergency SOS for train {} after forward failure; unexpected payload: {:?}",
                                        train_id, other
                                    );
                                    error!(
                                        "[{}::Station {}] {}",
                                        station_name_clone, station_id_clone, reason
                                    );
                                    telemetry.record(TelemetryEvent::EmergencySOSFailed {
                                        reason,
                                    });
                                }
                            }
                        }
                    }
                    return;
                }
                Err(mpsc::SendError(other)) => {
                    error!(
                        "[{}::Station {}] Failed to forward train {} to {} due to unexpected payload: {:?}",
                        station_name_clone, station_id_clone, train_id, next_stop, other
                    );
                    telemetry.record(TelemetryEvent::TrainForwardFailed {
                        reason: format!(
                            "Failed to forward train {} to {} due to unexpected payload",
                            train_id, next_stop
                        ),
                    });
                    return;
                }
            }

            // if next_stop_handle
            //     .send(StationCommand::ReceiveTrain {
            //         train,
            //         reply_to: transit_tx,
            //     })
            //     .is_err()
            // {
            //     error!(
            //         "[{}::Station {}] Failed to forward train {} to {}",
            //         station_name_clone, station_id_clone, train_id, next_stop
            //     );
            //     telemetry.record(TelemetryEvent::TrainForwardFailed { reason: format!("Failed to forward train {} to {}", train_id, next_stop) });
            //     if let Some(request_id) = train.request_id {
            //         if let Some(reply_to) = train.engine_request_reply_to.as_ref() {
            //             let _ = reply_to.send(StationCommand::EngineTransferFailed {
            //                 request_id,
            //                 mission_id: train.mission_id,
            //                 responder_id: station_id_clone,
            //                 reason: format!(
            //                     "Failed to forward train {} to {}: destination station unreachable",
            //                     train_id, next_stop
            //                 ),
            //             });
            //         }
            //     }
            //     if let Some(mission_id) = train.mission_id {
            //         if !train.cars.is_empty() {
            //             let _ = station_tx_clone.send(StationCommand::HandleEmergencySOS {
            //                 mission_id,
            //                 destination: train.destination,
            //                 surviving_cars: train.cars,
            //                 report_to: train.report_to,
            //             });
            //         }
            //     }
            //     return;
            // }

            let transit_ack_timeout = Duration::from_millis(STATION_HEARTBEAT_MS.saturating_mul(5));
            match transit_rx.recv_timeout(transit_ack_timeout) {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    let reason = format!(
                        "Next stop {} rejected train {} during intake: {:?}",
                        next_stop, train_id, e
                    );
                    warn!(
                        "[{}::Station {}] {}",
                        station_name_clone, station_id_clone, reason
                    );
                    telemetry.record(TelemetryEvent::TrainForwardFailed {
                        reason: reason.clone(),
                    });

                    if let Some((request_id, mission_id, reply_to)) = transfer_failure_target.as_ref()
                    {
                        let _ = reply_to.send(StationCommand::EngineTransferFailed {
                            request_id: *request_id,
                            mission_id: *mission_id,
                            responder_id: station_id_clone,
                            reason,
                        });
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let reason = format!(
                        "Timed out waiting for transit ACK for train {} to station {}",
                        train_id, next_stop
                    );
                    warn!(
                        "[{}::Station {}] {}",
                        station_name_clone, station_id_clone, reason
                    );
                    telemetry.record(TelemetryEvent::TrainForwardFailed {
                        reason: reason.clone(),
                    });

                    if let Some((request_id, mission_id, reply_to)) = transfer_failure_target.as_ref()
                    {
                        let _ = reply_to.send(StationCommand::EngineTransferFailed {
                            request_id: *request_id,
                            mission_id: *mission_id,
                            responder_id: station_id_clone,
                            reason,
                        });
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let reason = format!(
                        "Transit ACK channel disconnected for train {} to station {}",
                        train_id, next_stop
                    );
                    warn!(
                        "[{}::Station {}] {}",
                        station_name_clone, station_id_clone, reason
                    );
                    telemetry.record(TelemetryEvent::TrainForwardFailed {
                        reason: reason.clone(),
                    });

                    if let Some((request_id, mission_id, reply_to)) = transfer_failure_target.as_ref()
                    {
                        let _ = reply_to.send(StationCommand::EngineTransferFailed {
                            request_id: *request_id,
                            mission_id: *mission_id,
                            responder_id: station_id_clone,
                            reason,
                        });
                    }
                }
            }
        });
    }
}
