use rand::{Rng, thread_rng};
use std::f32::consts::PI;

use rapier3d::prelude::*;

use crate::model::{Instance, InstanceRaw};

const GRAVITY: Vector<f32> = vector![0.0, -9.81, 0.0];
const REI_SPAWN_TIME: f32 = 3.157 / 16.0;
pub const NUM_REIS: usize = 1000;

// https://www.youtube.com/watch?v=x4tw4CIuBks
#[derive(Default)]
pub struct PhysicsSimulation {
    collider_set: ColliderSet,
    rigidbody_set: RigidBodySet,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    reis: Vec<RigidBodyHandle>,
    timer: f32,
    rei_index: usize,
}

fn random_rotation() -> Vector<f32> {
    let mut rng = thread_rng();

    vector![
        rng.gen_range(0.0..6.18),
        rng.gen_range(0.0..6.18),
        rng.gen_range(0.0..6.18)
    ]
}

impl PhysicsSimulation {
    pub fn new() -> Self {
        let mut collider_set = ColliderSet::new();
        let mut rigidbody_set = RigidBodySet::new();

        let ground = ColliderBuilder::cuboid(1000.0, 0.1, 1000.0).build();
        collider_set.insert(ground);

        let rei = rigidbody_set.insert(
            RigidBodyBuilder::fixed()
                .translation(vector![0.0, 0.0, 0.0])
                .build(),
        );
        collider_set.insert_with_parent(rei_collider(), rei, &mut rigidbody_set);

        Self {
            collider_set,
            rigidbody_set,
            reis: Vec::with_capacity(NUM_REIS),
            ..Default::default()
        }
    }

    fn spawn_rei(&mut self) {
        let mut rng = thread_rng();

        let rei = self.rigidbody_set.insert(
            RigidBodyBuilder::dynamic()
            .translation(vector![rng.gen_range(-20.0..20.0), 10.0, rng.gen_range(-50.0..0.0)])
            .rotation(random_rotation())
            .build()
        );
        self.collider_set.insert_with_parent(rei_collider(), rei, &mut self.rigidbody_set);

        if self.reis.len() < NUM_REIS {
            self.reis.push(rei);
        } else {
            self.remove_rei(self.rei_index);
            self.reis[self.rei_index] = rei;
            self.rei_index = (self.rei_index + 1) % NUM_REIS;
        }
    }

    fn remove_rei(&mut self, rei_index: usize) {
        self.rigidbody_set.remove(self.reis[rei_index], 
            &mut self.island_manager, 
            &mut self.collider_set, 
            &mut self.impulse_joint_set, 
            &mut self.multibody_joint_set, 
            true 
        );
    }

    pub fn update(&mut self, delta_time: f32) {
        self.timer += delta_time;
        
        if self.timer >= REI_SPAWN_TIME {
            self.timer = 0.0;
            self.spawn_rei();
        }

        self.integration_parameters.dt = delta_time;

        self.physics_pipeline.step(
            &GRAVITY,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigidbody_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            &(),
            &(),
        );
    }

    pub fn instances(&self) -> Vec<InstanceRaw> {
        self.rigidbody_set
            .iter()
            .map(|(_, rb)| Instance::from_rapier_position(rb.position()).to_raw())
            .collect()
    }

    pub fn num_instances(&self) -> usize {
        self.reis.len() + 1
    }
}

fn rei_collider() -> rapier3d::prelude::Collider {
    let head_shape = SharedShape::round_cylinder(0.4, 0.95, 0.5);
    let body_shape = SharedShape::capsule_y(0.7, 0.65);

    let head_trans = Isometry::from_parts(
        Translation::new(0.0, 1.1, 0.0),
        Rotation::new(vector![1.0, 0.0, 0.0] * PI / 2.0),
    );
    let body_trans = Isometry::translation(0.0, 3.35, -0.1);

    ColliderBuilder::compound(vec![(head_trans, head_shape), (body_trans, body_shape)])
        .density(1.0)
        .restitution(0.8)
        .build()
}
