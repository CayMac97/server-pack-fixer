// Hook helper: streaming::Streaming::wait (HEAD)
fn server_pack_fixer_streaming_update(streaming: &mut Streaming, world: &mut World) {
    let s = crate::mod_server_pack_fixer::current_settings();
    if !s.enabled
        || !s.chunk_fixer
        || !streaming.remote
        || !streaming.mesh
        || streaming.paused
    {
        return;
    }
    let mut stranded = Vec::new();
    for (col, mask) in world.dirty_sections.columns() {
        stranded.push((col, mask));
    }
    if stranded.is_empty() {
        return;
    }
    let mut meshed_columns = 0u64;
    for ((x, z), sections) in stranded {
        if !streaming.lit.contains(&(x, z)) || streaming.lighting.contains_key(&(x, z)) {
            continue;
        }
        if !world.chunks.contains_key(&(x, z)) || !streaming.ranges().in_view(x, z) {
            continue;
        }
        // Only step in for columns that `Streaming::update` skipped because at least 1 of their 8
        // neighbour columns hasn't been sent by the server yet.
        let mut all_neighbours_ready = true;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let n = (x + dx, z + dz);
                if !streaming.lit.contains(&n)
                    || streaming.lighting.contains_key(&n)
                    || !world.chunks.contains_key(&n)
                {
                    all_neighbours_ready = false;
                }
            }
        }
        if all_neighbours_ready {
            continue;
        }
        let mut region: Region = Default::default();
        for dz in 0..3 {
            for dx in 0..3 {
                region[dz][dx] = world
                    .chunks
                    .get(&(x + dx as i32 - 1, z + dz as i32 - 1))
                    .cloned();
            }
        }
        if region[1][1].is_none() {
            continue;
        }
        let region = Box::new(region);
        world.dirty_sections.take_column(x, z);
        meshed_columns += 1;
        for sy_u in sections.iter() {
            let sy = sy_u as i32;
            let first = streaming.meshed.insert((x, z));
            streaming.sequence += 1;
            streaming.meshing.insert((x, sy, z), streaming.sequence);
            streaming.submit(
                Some((x, z)),
                if first { 2 } else { 0 },
                Job::Mesh(
                    x,
                    sy,
                    z,
                    region.clone(),
                    streaming.sequence,
                    MeshSettings {
                        nether: streaming.nether_shading,
                        zoom: streaming.biome_zoom,
                        blend_radius: streaming.biome_blend,
                        smooth_lighting: streaming.smooth_lighting,
                        cutout_leaves: streaming.cutout_leaves,
                    },
                ),
            );
        }
    }
    if meshed_columns > 0 {
        crate::mod_server_pack_fixer::add_partial_chunks_meshed(meshed_columns);
    }
}
