use glow::HasContext;
use khronos_egl as egl;

pub struct Context{
    instance:egl::Instance<egl::Dynamic<libloading::Library, egl::EGL1_5>>,
    display:egl::Display,
    config:egl::Config,
    egl_context:egl::Context,
    renderbuffer:glow::NativeRenderbuffer,
    framebuffer:glow::NativeFramebuffer,
    height:u32,
    width:u32,
    ctx:glow::Context
}

const EGL_CONFIG:&'static [egl::Int] = &[
    egl::SURFACE_TYPE, egl::PBUFFER_BIT,
    egl::RED_SIZE, 8,
    egl::GREEN_SIZE, 8,
    egl::BLUE_SIZE, 8,
    egl::ALPHA_SIZE, 8,
    egl::DEPTH_SIZE, 0,
    egl::LUMINANCE_SIZE, 0,
    egl::STENCIL_SIZE, 0,
    egl::COLOR_BUFFER_TYPE, egl::RGB_BUFFER,
    egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
    egl::NONE
];

impl Context{
    /// offscreen rendering
    pub fn create(libegl_path:&str, width:u32, height:u32) -> Self{
        let lib = unsafe{libloading::Library::new(libegl_path).expect("unable to find libEGL")};
        let egl = unsafe { egl::DynamicInstance::<egl::EGL1_5>::load_required_from(lib).expect("unable to load libEGL") };

        let display = egl.get_display(egl::DEFAULT_DISPLAY).expect("failed to get display");
        let (major, minor) = egl.initialize(display).expect("failed to initialize display");

        
        let config = egl.choose_first_config(display, EGL_CONFIG).unwrap().unwrap();

        let surface = egl.create_pbuffer_surface(display, config, &[
            egl::WIDTH, 1,
            egl::HEIGHT, 1,
            egl::NONE
        ]).unwrap();

        egl.bind_api(egl::OPENGL_ES_API).expect("faild to bind libGLES");

        let attr = [
            egl::CONTEXT_CLIENT_VERSION, 3,
            egl::NONE
        ];

        let egl_ctx = egl.create_context(display, config, None, &attr).unwrap();

        egl.make_current(display, Some(surface), Some(surface), Some(egl_ctx)).expect("failed to make egl surface.");

        let ctx = unsafe{glow::Context::from_loader_function(|name|{
            match egl.get_proc_address(name){
                Some(v) => {
                    #[cfg(test)]
                    println!("success load {}", name);
                    v as *const _
                },
                None => {
                    #[cfg(test)]
                    println!("failed to load {}", name);
                    0 as *const _
                }
            }
        })};

        unsafe{
            let rb = ctx.create_renderbuffer().unwrap();
            ctx.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
            ctx.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, width as i32, height as i32);
            ctx.bind_renderbuffer(glow::RENDERBUFFER, None);

            let framebuffer = ctx.create_framebuffer().unwrap();
            ctx.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            ctx.framebuffer_renderbuffer(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::RENDERBUFFER, Some(rb));

            Context { 
                instance:egl,
                display:display,
                config,
                egl_context:egl_ctx,
                renderbuffer:rb,
                framebuffer:framebuffer,
                width,
                height,
                ctx: ctx 
            }
        }

        
    }

    pub fn resize_framebuffer(&mut self, width:u32, height:u32){
        unsafe{
            let rb = self.ctx.create_renderbuffer().unwrap();
            let oldrb = self.ctx.get_parameter_i32(glow::RENDERBUFFER_BINDING);

            self.ctx.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
            self.ctx.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, width as i32, height as i32);
            self.ctx.bind_renderbuffer(glow::RENDERBUFFER, None);

            let frame = self.ctx.create_framebuffer().unwrap();
            let old_draw_frame = self.ctx.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING);
            let old_read_frame = self.ctx.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING);

            self.ctx.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(frame));
            self.ctx.framebuffer_renderbuffer(glow::DRAW_FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::RENDERBUFFER, Some(rb));

            if oldrb != 0{
                // binds the old renderbuffer
                self.ctx.bind_renderbuffer(glow::RENDERBUFFER, Some(std::mem::transmute(oldrb)));
            }

            
            let old_framebuffer = std::mem::transmute(self.framebuffer);

            if old_read_frame == old_framebuffer && old_draw_frame == old_framebuffer{
                // update both framebuffers
                self.ctx.bind_framebuffer(glow::FRAMEBUFFER, Some(frame));

            } else if old_draw_frame == old_framebuffer{
                // update draw framebuffer
                self.ctx.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(frame));
                
            } else if old_read_frame == old_framebuffer{
                // update read framebuffer
                self.ctx.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(frame));

            }
            
            if old_draw_frame != old_framebuffer{
                // not the default frame buffer, binds the old binded framebuffer
                self.ctx.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(std::mem::transmute(old_draw_frame)));
            } 

            self.ctx.delete_framebuffer(self.framebuffer);
            self.ctx.delete_renderbuffer(self.renderbuffer);

            self.framebuffer = frame;
            self.renderbuffer = rb;
            
        };
        self.width = width;
        self.height = height;
    }

    /// format RGBA8
    pub fn copy_pixels(&self, v:&mut [u8]){
        unsafe{
            let buf = self.ctx.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING);

            if buf != std::mem::transmute(self.framebuffer){
                // bind to the default framebuffer
                self.ctx.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.framebuffer));
            }

            self.ctx.read_pixels(0, 0, self.width as i32, self.height as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(v));

            if buf != std::mem::transmute(self.framebuffer){
                self.ctx.bind_framebuffer(glow::READ_FRAMEBUFFER, std::mem::transmute(buf));
            }
        }
    }

    /// overide the bind framebuffer to bind default framebuffer
    pub unsafe fn bind_framebuffer(&self, target:u32, framebuffer:Option<glow::NativeFramebuffer>) {
        if framebuffer.is_none(){
            self.ctx.bind_framebuffer(target, Some(self.framebuffer));
        } else{
            self.ctx.bind_framebuffer(target, framebuffer);
        }
    }

    /// overide
    pub unsafe fn get_parameter_i32(&self, parameter:u32) -> i32{
        let re = self.ctx.get_parameter_i32(parameter);

        let d = std::mem::transmute(self.framebuffer);
        let is_default = parameter == glow::FRAMEBUFFER_BINDING || parameter == glow::READ_FRAMEBUFFER_BINDING || parameter == glow::DRAW_FRAMEBUFFER_BINDING;

        if is_default && re == d{
            return 0
        }
        return re;
    }

    /// overide
    pub unsafe fn get_parameter_f32(&self, parameter:u32) -> f32{
        let re = self.ctx.get_parameter_f32(parameter);

        let d = std::mem::transmute::<_, i32>(self.framebuffer) as f32;
        let is_default = parameter == glow::FRAMEBUFFER_BINDING || parameter == glow::READ_FRAMEBUFFER_BINDING || parameter == glow::DRAW_FRAMEBUFFER_BINDING;

        if is_default && re == d{
            return 0.0
        }
        return re;
    }
}

impl AsRef<glow::Context> for Context{
    fn as_ref(&self) -> &glow::Context {
        &self.ctx
    }
}

impl AsMut<glow::Context> for Context{
    fn as_mut(&mut self) -> &mut glow::Context {
        &mut self.ctx
    }
}

impl std::ops::Deref for Context{
    type Target = glow::Context;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl Drop for Context{
    fn drop(&mut self) {
        match self.instance.terminate(self.display){
            _ => {}
        };
    }
}



#[test]
fn test_display(){
    const data:[f32;9] = [
        -1.0, -1.0, 0.0,
        1.0, -1.0, 0.0,
        0.0,  1.0, 0.0,
    ];

    let ctx = Context::create("libEGL.dll", 100, 100);
    unsafe{
        ctx.clear_color(1.0, 0.0, 0.0, 1.0);
        ctx.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

        let a = ctx.create_vertex_array().unwrap();
        ctx.bind_vertex_array(Some(a));
        

        let buf = ctx.create_buffer().unwrap();
        ctx.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
        ctx.buffer_data_u8_slice(glow::ARRAY_BUFFER, std::slice::from_raw_parts(&data as *const f32 as *const u8, 9*4), glow::STATIC_DRAW);
        ctx.enable_vertex_attrib_array(0);
        ctx.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
        
        ctx.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0);
        ctx.draw_arrays(glow::TRIANGLES, 0, 3);
        ctx.disable_vertex_attrib_array(0);
    };
    
    let mut v = Vec::new();
    v.resize(100*100 * 4, 0u8);

    unsafe{ctx.read_pixels(0, 0, 100, 100, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(&mut v))};

    let r = image::RgbaImage::from_vec(100, 100, v).unwrap();
    r.save_with_format("a.png", image::ImageFormat::Png).unwrap();
}