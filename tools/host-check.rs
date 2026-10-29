//! Packed guest lifecycle checks using native drawing admission; no rendered frame.
use mod_host::{ModHost,ModGrants,WorldObservation,GameplaySnapshot,GameplayVector3,TerrainSource,TerrainCheck,TerrainQuery,TerrainRegion,TerrainRefresh,TerrainBlockState,TerrainShapeBinding,TerrainShape,TerrainStamp,TerrainScheduler};

struct Fixture {yaw:f32,pitch:f32,alive:bool,dimension:i32,session:u64,wall:bool,unknown:bool,peak:u64,position:[f32;3],camera:Option<[f32;3]>,face_frames:u32,face_run:u32,face_ended:bool}
impl Fixture {
    fn world(&self)->WorldObservation{WorldObservation{session:self.session,dimension:self.dimension,world:"herobrine-fixture".into(),position:self.position.into(),alive:self.alive,runtime_id:1}}
}
impl TerrainSource for Fixture {
    fn check(&mut self,_:&TerrainQuery,_:Option<&TerrainRegion>)->TerrainCheck{TerrainCheck::Stale{cells:1024}}
    fn sample(&mut self,query:&TerrainQuery)->TerrainRefresh{
        let mut states=vec![];
        for x in query.min.x..query.max.x{for z in query.min.z..query.max.z{for y in query.min.y..query.max.y{
            let wall=self.wall&&(x.abs()==2||z.abs()==2)&&y>=64;
            states.push(u16::from(y<64||wall));
        }}}
        let block=|identifier:&str,shape|TerrainBlockState{runtime_id:shape,identifier:Some(identifier.into()),canonical_state:None,tags:vec![],physics:None,collision:TerrainShapeBinding::Fixed(TerrainShape::Known(shape)),selection:TerrainShapeBinding::NotRequested};
        TerrainRefresh::Replace(Box::new(TerrainRegion{stamp:TerrainStamp{session:self.session,dimension:self.dimension,registry:1,revision:0},chunks:vec![],layer_count:1,
            loaded:vec![if self.unknown{0}else{u64::MAX};states.len().div_ceil(64)],states,palette:vec![block("minecraft:air",0),block("minecraft:stone",1)],shape_offsets:vec![0,0,1],shape_boxes:vec![0.,0.,0.,1.,1.,1.],overrides:vec![]}))
    }
}
fn tick(host:&mut ModHost,fixture:&mut Fixture,scheduler:&mut TerrainScheduler)->anyhow::Result<()> {
    scheduler.refresh(&mut [host],8192,fixture);
    host.observe_presentation(Some(fixture.world()),None,None,vec![])?;
    host.observe_gameplay(mod_host::GameplayFacts {camera:fixture.camera.map(|p|mod_host::GameplayCameraPose{position:GameplayVector3{x:p[0],y:p[1],z:p[2]},yaw:fixture.yaw,pitch:fixture.pitch}),..Default::default()})?;
    host.frame_with_gameplay(false,Some(GameplaySnapshot{session:17,dimension:fixture.dimension,eye:GameplayVector3{x:fixture.position[0],y:fixture.position[1]+1.6,z:fixture.position[2]},yaw:fixture.yaw,pitch:fixture.pitch,attack_held:false,frame_seconds:0.05,players:vec![]}))?;
    fixture.peak=fixture.peak.max(host.last_fuel_used());
    // Only emulate GPU admission. This does not render or prove pixel output.
    for resources in &host.render().0.retained {
        for clip in resources.clips.values() { clip.status.complete(Ok(())); }
        for instance in resources.instances.values() {
            anyhow::ensure!(instance.clip.is_some(),"model has no private depth buffer");
            anyhow::ensure!(resources.clips.contains_key(&instance.clip.unwrap()),"model references missing clip");
        }
    }
    let active=face_active(host);
    if active {fixture.face_frames+=1;fixture.face_run+=1;anyhow::ensure!(fixture.face_run<=5,"face jumpscare exceeds configured duration");}
    else {fixture.face_ended|=fixture.face_run>0;fixture.face_run=0;}
    anyhow::ensure!(host.is_active()&&!host.is_recovering(),"guest inactive: {:?}",host.label());Ok(())
}
fn face_active(host:&ModHost)->bool {host.render().0.passes.iter().any(|p|p.name=="herobrine-face"&&p.enabled)}
fn figure(host:&ModHost)->Option<[f32;3]>{
    host.render().0.retained.iter().flat_map(|r|r.instances.values()).find_map(|i|match i.anchor{
        mod_render::retained::Anchor::World{origin,..}=>Some(origin.map(|v|v as f32)),_=>None})
}
fn wait_spawn(host:&mut ModHost,fixture:&mut Fixture,scheduler:&mut TerrainScheduler)->anyhow::Result<[f32;3]>{
    for _ in 0..180{tick(host,fixture,scheduler)?;if let Some(feet)=figure(host){return Ok(feet);}}
    anyhow::bail!("figure never appeared: {:?}",host.label())
}
fn look(fixture:&mut Fixture,feet:[f32;3]){
    let d=[feet[0]-fixture.position[0],feet[1]+0.95-fixture.position[1]-1.6,feet[2]-fixture.position[2]];fixture.yaw=(-d[0]).atan2(-d[2]);fixture.pitch=(d[1]/d.iter().map(|v|v*v).sum::<f32>().sqrt()).asin();
}
fn main()->anyhow::Result<()> {
    let path=std::env::args_os().nth(1).ok_or_else(||anyhow::anyhow!("supply component with fast test companion"))?;
    let mut host=ModHost::load_with_grants(std::path::Path::new(&path),ModGrants{terrain:true,player_state:true,world_draw:true,settings:true,hud:true,render_post:true,..Default::default()})?;
    let mut f=Fixture{yaw:0.,pitch:0.,alive:true,dimension:0,session:2,wall:false,unknown:false,peak:0,position:[0.5,64.,0.5],camera:None,face_frames:0,face_run:0,face_ended:false};let mut scheduler=TerrainScheduler::default();
    let feet=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    anyhow::ensure!((feet[1]-64.).abs()<0.01,"floating placement");
    f.camera=Some([feet[0]+(feet[0]-f.position[0])*1.5,65.6,feet[2]+(feet[2]-f.position[2])*1.5]);
    tick(&mut host,&mut f,&mut scheduler)?;
    let dx=f.position[0]-feet[0];let dz=f.position[2]-feet[2];let len=(dx*dx+dz*dz).sqrt();
    for instance in host.render().0.retained.iter().flat_map(|r|r.instances.values()) {
        let q=instance.transform.rotation;let forward=[2.*q[1]*q[3],1.-2.*q[1]*q[1]];
        anyhow::ensure!((forward[0]*dx+forward[1]*dz)/len>0.999,"Herobrine faces the third-person camera instead of the player");
    }
    f.camera=None;
    let geometry=host.render().0.retained.iter().flat_map(|r|r.geometry.keys().copied()).collect::<Vec<_>>();
    for _ in 0..3{tick(&mut host,&mut f,&mut scheduler)?;}
    anyhow::ensure!(figure(&host).is_some(),"vanished without gaze");
    f.wall=true;look(&mut f,feet);
    for _ in 0..6{tick(&mut host,&mut f,&mut scheduler)?;}
    anyhow::ensure!(figure(&host).is_some(),"gaze passed through wall");
    f.wall=false;
    for _ in 0..4{tick(&mut host,&mut f,&mut scheduler)?;}
    anyhow::ensure!(figure(&host).is_none(),"looking at Herobrine did not remove him");
    let _=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    let next_geometry=host.render().0.retained.iter().flat_map(|r|r.geometry.keys().copied()).collect::<Vec<_>>();
    anyhow::ensure!(geometry==next_geometry,"geometry regenerated between scares");
    let before_pause=figure(&host);
    host.observe_presentation(Some(f.world()),None,None,vec![])?;host.frame(false)?;
    anyhow::ensure!(figure(&host)==before_pause,"pause removed apparition");
    anyhow::ensure!(!face_active(&host),"pause retained face overlay");
    look(&mut f,before_pause.unwrap());for _ in 0..6{tick(&mut host,&mut f,&mut scheduler)?;}
    anyhow::ensure!(figure(&host).is_none(),"resumed gaze did not remove apparition");
    f.unknown=true;for _ in 0..100{tick(&mut host,&mut f,&mut scheduler)?;}
    anyhow::ensure!(figure(&host).is_none(),"spawned in unknown terrain");
    f.unknown=false;let _=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    f.alive=false;tick(&mut host,&mut f,&mut scheduler)?;anyhow::ensure!(figure(&host).is_none(),"death retained apparition");
    anyhow::ensure!(!face_active(&host),"death retained face overlay");
    anyhow::ensure!(host.render().0.retained.iter().all(|r|r.clips.is_empty()&&r.geometry.is_empty()),"death leaked mask or geometry");
    f.alive=true;let _=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    f.dimension=1;tick(&mut host,&mut f,&mut scheduler)?;anyhow::ensure!(figure(&host).is_none(),"dimension switch retained apparition");
    let _=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    f.session+=1;tick(&mut host,&mut f,&mut scheduler)?;anyhow::ensure!(figure(&host).is_none(),"new connection retained apparition");
    let persistent=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    for _ in 0..400{tick(&mut host,&mut f,&mut scheduler)?;anyhow::ensure!(figure(&host)==Some(persistent),"unnoticed apparition expired");}
    f.position[0]+=50.;tick(&mut host,&mut f,&mut scheduler)?;
    anyhow::ensure!(figure(&host).is_none(),"distant apparition did not disappear");
    let distant=wait_spawn(&mut host,&mut f,&mut scheduler)?;
    f.position=[distant[0],64.,distant[2]+20.];look(&mut f,distant);
    for _ in 0..6{tick(&mut host,&mut f,&mut scheduler)?;}
    anyhow::ensure!(figure(&host).is_none(),"direct gaze beyond one query extent did not remove the apparition");
    anyhow::ensure!(f.face_frames>0&&f.face_ended,"face effect did not start and end: {:?}",host.label());
    anyhow::ensure!(host.render().0.passes.iter().any(|p|p.name=="herobrine-face"&&!p.depth),"face shader not admitted");
    println!("PASS packed WASM: 3D clip admission, character facing, solid ground, gaze removal, wall occlusion, geometry reuse, pause, unknown chunks, death/dimension/connection cleanup, face shader and timed jumpscares. Peak frame fuel={}/200000. Native admission only; no game frame rendered.",f.peak);
    Ok(())
}
