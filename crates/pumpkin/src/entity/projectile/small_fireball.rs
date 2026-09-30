use std::sync::atomic::{AtomicBool, Ordering};

use pumpkin_util::math::vector3::Vector3;

use crate::{
    block::blocks::fire::{FireBlockBase, fire::FireBlock},
    entity::{
        Entity, EntityBase,
        projectile::{ProjectileHit, ThrownItemEntity, fireball::INITIAL_ACCELERATION_POWER},
    },
    server::Server,
};

const GRAVITY: f64 = 0.0;

pub struct SmallFireballEntity {
    pub thrown: ThrownItemEntity,
}

impl SmallFireballEntity {
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        let thrown = ThrownItemEntity {
            entity,
            owner_id: None,
            collides_with_projectiles: false,
            has_hit: AtomicBool::new(false),
            gravity: GRAVITY,
        };

        Self { thrown }
    }

    #[must_use]
    pub fn new_shot(entity: Entity, shooter: &Entity, direction: Vector3<f64>) -> Self {
        let thrown = ThrownItemEntity::new(entity, shooter, GRAVITY);
        let accel = INITIAL_ACCELERATION_POWER;
        thrown
            .entity
            .velocity
            .store(direction.normalize().multiply(accel, accel, accel));
        Self { thrown }
    }
}

impl EntityBase for SmallFireballEntity {
    fn get_owner_id(&self) -> Option<i32> {
        self.thrown.owner_id
    }

    fn tick(&self, caller: &dyn EntityBase, _server: &Server) {
        self.thrown
            .process_fireball_tick(caller, INITIAL_ACCELERATION_POWER);
    }

    fn get_entity(&self) -> &Entity {
        self.thrown.get_entity()
    }

    fn get_living_entity(&self) -> Option<&crate::entity::living::LivingEntity> {
        None
    }
    fn cast_any(&self) -> &dyn std::any::Any {
        self
    }

    fn on_hit(&self, hit: ProjectileHit) {
        match hit {
            ProjectileHit::Entity { ref entity, .. } => {
                let target = entity.get_entity();
                let previous_fire_ticks = target.fire_ticks.load(Ordering::Relaxed);
                target.set_on_fire_for(5.0);
                let world = self.get_entity().world.load();
                let owner = self
                    .thrown
                    .owner_id
                    .and_then(|id| world.get_entity_by_id(id));
                if !entity.damage_with_context(
                    entity.as_ref(),
                    5.0,
                    pumpkin_data::damage::DamageType::FIREBALL,
                    None,
                    Some(self.get_entity()),
                    owner.as_deref(),
                ) {
                    target
                        .fire_ticks
                        .store(previous_fire_ticks, Ordering::Relaxed);
                }
            }
            ProjectileHit::Block { pos, face, .. } => {
                let world = self.get_entity().world.load();
                let owner_is_mob = self
                    .thrown
                    .owner_id
                    .and_then(|id| world.get_entity_by_id(id))
                    .is_some_and(|owner| owner.get_mob().is_some());
                if owner_is_mob && !world.level_info.load().game_rules.mob_griefing {
                    return;
                }

                let block_to_place = match face {
                    pumpkin_data::BlockDirection::Up => pos.up(),
                    pumpkin_data::BlockDirection::Down => pos.down(),
                    pumpkin_data::BlockDirection::North => pos.north(),
                    pumpkin_data::BlockDirection::South => pos.south(),
                    pumpkin_data::BlockDirection::West => pos.west(),
                    pumpkin_data::BlockDirection::East => pos.east(),
                };
                if !world.get_block_state(&block_to_place).is_air() {
                    return;
                }

                let fire_block = FireBlockBase::get_fire_type(&world, &block_to_place);
                let fire_state = if fire_block == pumpkin_data::Block::FIRE {
                    FireBlock.get_state_for_position(&world, &fire_block, &block_to_place)
                } else {
                    fire_block.default_state.id
                };
                world.set_block_state(
                    &block_to_place,
                    fire_state,
                    pumpkin_world::world::BlockFlags::NOTIFY_ALL,
                );
            }
        }
    }
}
