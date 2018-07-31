use amethyst_assets::{
    Asset, AssetStorage, Handle, Loader, ProcessingState, Result as AssetsResult,
};
use amethyst_core::specs::prelude::{
    Component, Entity, EntityBuilder, Read, ReadExpect, VecStorage, WriteStorage,
};
use error::Result;
use mesh::{Mesh, MeshHandle};
use shape::Shape;
use std::marker::Sized;
use tex::TextureHandle;
use {Material, MaterialDefaults, PosTex, TextureOffset};

/// An asset handle to sprite sheet metadata.
pub type SpriteSheetHandle = Handle<SpriteSheet>;

/// Meta data for a sprite sheet texture.
///
/// Contains a handle to the texture and the sprite coordinates on the texture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpriteSheet {
    /// Index into `MaterialTextureSet` of the texture for this sprite sheet.
    pub texture_id: u64,
    /// A list of sprites in this sprite sheet.
    pub sprites: Vec<Sprite>,
}

impl Asset for SpriteSheet {
    const NAME: &'static str = "renderer::SpriteSheet";
    type Data = Self;
    type HandleStorage = VecStorage<Handle<Self>>;
}

impl From<SpriteSheet> for AssetsResult<ProcessingState<SpriteSheet>> {
    fn from(sprite_sheet: SpriteSheet) -> AssetsResult<ProcessingState<SpriteSheet>> {
        Ok(ProcessingState::Loaded(sprite_sheet))
    }
}

/// Dimensions and texture coordinates of each sprite in a sprite sheet.
///
/// The texture coordinates should be normalized to a value between 0.0 and 1.0:
///
/// * X axis: 0.0 is the left side and 1.0 is the right side.
/// * Y axis: 0.0 is the top and 1.0 is the bottom.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    /// Pixel width of the sprite
    pub width: f32,
    /// Pixel height of the sprite
    pub height: f32,
    /// Normalized left x coordinate
    pub left: f32,
    /// Normalized right x coordinate
    pub right: f32,
    /// Normalized top y coordinate
    pub top: f32,
    /// Normalized bottom y coordinate
    pub bottom: f32,
}

impl From<((f32, f32), (f32, f32), (f32, f32))> for Sprite {
    fn from(
        ((width, height), (left, right), (top, bottom)): ((f32, f32), (f32, f32), (f32, f32)),
    ) -> Self {
        Sprite {
            width,
            height,
            left,
            right,
            top,
            bottom,
        }
    }
}

impl From<[f32; 6]> for Sprite {
    fn from(uv: [f32; 6]) -> Self {
        Sprite {
            width: uv[0],
            height: uv[1],
            left: uv[2],
            right: uv[3],
            top: uv[4],
            bottom: uv[5],
        }
    }
}

/// Information for rendering a sprite.
///
/// Instead of using a `Mesh` on a `DrawFlat` render pass, we can use a simpler set of shaders to
/// render sprites. This struct carries the information necessary for the sprite pass.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteRenderInfo {
    /// Handle to the sprite sheet of the sprite
    pub sprite_sheet: SpriteSheetHandle,
    /// Index of the sprite on the sprite sheet
    pub sprite_number: usize,
}

impl Component for SpriteRenderInfo {
    type Storage = VecStorage<Self>;
}

/// SystemData containing the data necessary to handle new rendered sprites
#[derive(SystemData)]
pub struct SpriteRenderData<'a> {
    /// Storage containing the meshes
    pub meshes: WriteStorage<'a, MeshHandle>,
    /// Storage containing the materials
    pub materials: WriteStorage<'a, Material>,
    /// Material defaults
    pub material_defaults: ReadExpect<'a, MaterialDefaults>,
    /// Asset loader
    pub loader: ReadExpect<'a, Loader>,
    /// Mesh asset storage
    pub asset_storage: Read<'a, AssetStorage<Mesh>>,
}

impl<'a> SpriteRenderData<'a> {
    /// Creates a MeshHandle and Material from the sprite and texture data.
    /// Useful if you plan on re-using the same sprite a lot and don't want to
    /// load the assets each time.
    pub fn build_mesh_and_material(
        &mut self,
        sprite: &Sprite,
        texture: TextureHandle,
        size: (f32, f32),
    ) -> (MeshHandle, Material) {
        let half_width = (sprite.right - sprite.left) * 0.5;
        let half_height = (sprite.bottom - sprite.top) * 0.5;

        let vertices =
            Shape::Plane(None).generate::<Vec<PosTex>>(Some((half_width, half_height, 0.0)));
        let mesh = self
            .loader
            .load_from_data(vertices, (), &self.asset_storage);

        let material = Material {
            albedo: texture,
            albedo_offset: TextureOffset {
                u: (sprite.left / size.0, sprite.right / size.0),
                v: (1.0 - sprite.bottom / size.1, 1.0 - sprite.top / size.1),
            },
            ..self.material_defaults.0.clone()
        };

        (mesh, material)
    }

    /// Adds a mesh and a material to an entity corresponding to the sprite and texture given.
    /// Note that is you need to insert the same sprite and texture, using ``add_multiple`` allows for better performances.
    pub fn add(
        &mut self,
        entity: Entity,
        sprite: &Sprite,
        texture: TextureHandle,
        texture_size: (f32, f32),
    ) -> Result<()> {
        let (mesh, material) = self.build_mesh_and_material(sprite, texture, texture_size);
        self.meshes.insert(entity, mesh)?;
        self.materials.insert(entity, material)?;
        Ok(())
    }

    /// Adds the same mesh and material to multiple entities corresponding to the sprite and texture given.
    pub fn add_multiple(
        &mut self,
        entities: Vec<Entity>,
        sprite: &Sprite,
        texture: TextureHandle,
        texture_size: (f32, f32),
    ) -> Result<()> {
        let len = entities.len();
        if len != 0 {
            let (mesh, material) = self.build_mesh_and_material(sprite, texture, texture_size);
            for entity in 0..len - 1 {
                self.meshes.insert(entities[entity], mesh.clone())?;
                self.materials.insert(entities[entity], material.clone())?;
            }
            self.meshes.insert(entities[len - 1], mesh)?;
            self.materials.insert(entities[len - 1], material)?;
        }
        Ok(())
    }
}

/// An easy way to attach and display a sprite when building an entity
pub trait WithSpriteRender
where
    Self: Sized,
{
    /// Adds a mesh and a material to the entity being built corresponding to the sprite and texture given.
    fn with_sprite(
        self,
        sprite: &Sprite,
        texture: TextureHandle,
        texture_size: (f32, f32),
    ) -> Result<Self>;
}

impl<'a> WithSpriteRender for EntityBuilder<'a> {
    fn with_sprite(
        self,
        sprite: &Sprite,
        texture: TextureHandle,
        texture_size: (f32, f32),
    ) -> Result<Self> {
        self.world.system_data::<SpriteRenderData>().add(
            self.entity,
            sprite,
            texture,
            texture_size,
        )?;
        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::Sprite;

    #[test]
    fn sprite_from_tuple_maps_fields_correctly() {
        assert_eq!(
            Sprite {
                width: 10.,
                height: 20.,
                left: 0.,
                right: 0.5,
                top: 0.75,
                bottom: 1.0,
            },
            ((10., 20.), (0.0, 0.5), (0.75, 1.0)).into()
        );
    }

    #[test]
    fn sprite_from_slice_maps_fields_correctly() {
        assert_eq!(
            Sprite {
                width: 10.,
                height: 20.,
                left: 0.,
                right: 0.5,
                top: 0.75,
                bottom: 1.0,
            },
            [10., 20., 0.0, 0.5, 0.75, 1.0].into()
        );
    }
}
