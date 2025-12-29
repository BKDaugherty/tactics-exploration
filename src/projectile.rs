use bevy::prelude::*;

use crate::combat::CombatAnimationId;

#[derive(Component)]
pub struct ProjectileQuadraticBezier {
    pub start: Vec3,
    pub control: Vec3,
    pub end: Vec3,
    pub t: f32,
    pub duration: f32,
    pub combat_anim_id: CombatAnimationId,
    pub ae_entity: Entity,
}

pub fn spawn_arrow(
    commands: &mut Commands,
    combat_anim_id: CombatAnimationId,
    ae_entity: Entity,
    start: Vec3,
    end: Vec3,
    image_handle: Handle<Image>,
) {
    let mid = start.midpoint(end);

    // TODO: Make this some function of the distance maybe?
    // Or maybe a part of the skill? It'd be nice if curves in general could be a part of the skill system?
    // could probably enum it up.
    let arc_height = 150.;
    let control = mid + Vec3::Y * arc_height;

    commands.spawn((
        Transform::from_translation(start),
        ProjectileQuadraticBezier {
            start,
            control,
            end,
            t: 0.0,
            duration: 1.0,
            combat_anim_id,
            ae_entity,
        },
        Sprite {
            image: image_handle,
            ..Default::default()
        },
    ));
}

// I could make this an enum of possible curves?
// And then one of them even could just be show up maybe to unify
// our two projectile systems?
fn quadratic_bezier(p0: Vec3, p1: Vec3, p2: Vec3, t: f32) -> Vec3 {
    let one_minus_t = 1.0 - t;

    // B(t) = (1 − t)² P0 + 2(1 − t)t P1 + t² P2
    (one_minus_t.powi(2) * p0) + (2.0 * one_minus_t * t * p1) + (t.powi(2) * p2)
}

#[derive(Message)]
pub struct ProjectileArrived {
    pub entity: Entity,
    pub combat_anim_id: CombatAnimationId,
    pub ae_entity: Entity,
}

pub fn projectile_arrival_system(
    mut commands: Commands,
    mut writer: MessageWriter<ProjectileArrived>,
    query: Query<(Entity, &ProjectileQuadraticBezier)>,
) {
    for (entity, bezier) in &query {
        if bezier.t >= 1.0 {
            writer.write(ProjectileArrived {
                entity,
                combat_anim_id: bezier.combat_anim_id.clone(),
                ae_entity: bezier.ae_entity.clone(),
            });
            commands.entity(entity).despawn();
        }
    }
}

pub fn projectile_bezier_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut ProjectileQuadraticBezier)>,
) {
    for (mut transform, mut bezier) in &mut query {
        bezier.t += time.delta_secs() / bezier.duration;
        let t = bezier.t.clamp(0.0, 1.0);

        let pos = quadratic_bezier(bezier.start, bezier.control, bezier.end, t);

        transform.translation = pos;

        let next_t = (t + 0.01).min(1.0);
        let next_pos = quadratic_bezier(bezier.start, bezier.control, bezier.end, next_t);
        let dir = (next_pos - pos).normalize();

        transform.rotation = Quat::from_rotation_arc(Vec3::X, dir);
    }
}
