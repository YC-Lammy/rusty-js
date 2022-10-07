pub struct DynamicBuffer(Vec<u8>);

impl DynamicBuffer{
    pub fn len(&self) -> usize{
        self.0.len()
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8]{
        &mut self.0
    }

    pub fn resize(&mut self, len:usize, v:u8){
        self.0.resize(len, v);
    }
    
    pub fn push_u8(&mut self, v:u8){
        self.0.push(v);
    }

    pub fn replace_u8(&mut self, index:usize, v:u8){
        self.0[index] = v;
    }

    pub fn insert_u8(&mut self, index:usize, v:u8){
        if self.len() <= index{
            return
        }

        self.0.insert(index, v);
    }

    pub fn push_u16(&mut self, v:u16){
        let v:[u8;2] = unsafe{std::mem::transmute(v)};
        self.0.extend(v);
    }

    pub fn push_u32(&mut self, v:u32){
        let v:[u8;4] = unsafe{std::mem::transmute(v)};
        self.0.extend(v);
    }

    pub fn replace_u32(&mut self, idx:usize, v:u32){
        let v:[u8;4] = unsafe{std::mem::transmute(v)};
        let s = & mut self.0[idx..];
        s[0] = v[0];
        s[1] = v[1];
        s[2] = v[2];
        s[3] = v[3];
    }

    pub fn insert_u32(&mut self, idx:usize, v:u32){
        self.0.resize(self.0.len() + 4, 0);
        self.0.copy_within(idx.., idx+4);
        
        let v:[u8;4] = unsafe{std::mem::transmute(v)};

        self.0[idx] = v[0];
        self.0[idx+1] = v[1];
        self.0[idx+2] = v[2];
        self.0[idx+3] = v[3];
    }

    pub fn push_u64(&mut self, v:u64){
        let v:[u8;8] = unsafe{std::mem::transmute(v)};
        self.0.extend(v);
    }

    pub fn push_char(&mut self, c:char){
        self.push_u32(c as u32)
    }

    pub fn push_bool(&mut self, v:bool){
        self.push_u8(v as u8)
    }

    pub fn insert_bytes(&mut self, index:usize, b:&[u8]){
        let l = self.0.len();
        self.0.resize(l + b.len(), 0);
        self.0.copy_within(index..l, index + 1);
        (&mut self.0[index..index + b.len()]).copy_from_slice(b);
    }

    pub fn iter<'a>(&'a self) -> DynamicBufferIterator<'a>{
        DynamicBufferIterator { 
            buffer: self.0.as_slice()
        }
    }
}

pub struct DynamicBufferIterator<'a>{
    buffer:&'a [u8]
}

impl<'a> DynamicBufferIterator<'a>{
    /// decrease the count in bytes
    pub unsafe fn decrease(&mut self, count:usize) {
        let len = self.buffer.len();
        self.buffer = std::slice::from_raw_parts(self.buffer.as_ptr().sub(count), len+count);
    }

    pub fn get_next_u8(&mut self) -> Option<u8>{
        let v = *self.buffer.get(0)?;
        self.buffer = &self.buffer[1..];
        return Some(v);
    }

    pub fn get_next_u16(&mut self) -> Option<u16>{
        let a = self.get_next_u8()?;
        let b = self.get_next_u8()?;
        return Some(unsafe{std::mem::transmute([a,b])})
    }

    pub fn get_next_u32(&mut self) -> Option<u32>{
        let a = self.get_next_u8()?;
        let b = self.get_next_u8()?;
        let c = self.get_next_u8()?;
        let d = self.get_next_u8()?;
        return Some(unsafe{std::mem::transmute([a,b,c,d])})
    }

    pub fn get_next_u64(&mut self) -> Option<u64>{
        let a = self.get_next_u8()?;
        let b = self.get_next_u8()?;
        let c = self.get_next_u8()?;
        let d = self.get_next_u8()?;
        let e = self.get_next_u8()?;
        let f = self.get_next_u8()?;
        let g = self.get_next_u8()?;
        let h = self.get_next_u8()?;
        return Some(unsafe{std::mem::transmute([a,b,c,d,e,f,g,h])})
    }

    pub fn get_next_bool(&mut self) -> Option<bool>{
        let v = self.get_next_u8()?;
        Some(v != 0)
    }
}