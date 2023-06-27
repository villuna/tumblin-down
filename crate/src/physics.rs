use std::f32::consts::PI;

use rapier3d::prelude::*;

use crate::model::{Instance, InstanceRaw};

const GRAVITY: Vector<f32> = vector![0.0, -9.81, 0.0];

// https://www.youtube.com/watch?v=x4tw4CIuBks
#[derive(Default)]
struct PhysicsSimulation {
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
}

impl PhysicsSimulation {
    pub fn new() -> Self {
        let mut collider_set = ColliderSet::new();
        let mut rigidbody_set = RigidBodySet::new();

        let ground = ColliderBuilder::cuboid(100.0, 0.1, 100.0).build();
        collider_set.insert(ground);

        let rei = rigidbody_set.insert(
            RigidBodyBuilder::dynamic()
                .translation(vector![0.0, 5.0, 0.0])
                .build(),
        );
        collider_set.insert_with_parent(rei_collider(), rei, &mut rigidbody_set);

        Self {
            collider_set,
            rigidbody_set,
            ..Default::default()
        }
    }

    pub fn update(&mut self, delta_time: f32) {
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
        .build()
}
