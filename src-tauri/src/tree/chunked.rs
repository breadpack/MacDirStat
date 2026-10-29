use std::ops::{Index, IndexMut};

const CHUNK_BITS: usize = 16;
const CHUNK: usize = 1 << CHUNK_BITS;

/// 고정 크기 블록 단위로 커지는 Vec.
/// 일반 Vec은 용량이 찰 때마다 두 배 크기로 재할당·복사하므로 그 순간 메모리가 1.5~2배로 튄다.
/// 블록 단위로 할당하면 복사가 없고 여유 공간도 블록 하나 이하로 유지된다.
pub struct ChunkedVec<T> {
    chunks: Vec<Vec<T>>,
    len: usize,
}

impl<T> ChunkedVec<T> {
    pub fn new() -> Self {
        Self { chunks: Vec::new(), len: 0 }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push(&mut self, value: T) -> usize {
        if self.chunks.last().is_none_or(|c| c.len() == CHUNK) {
            self.chunks.push(Vec::with_capacity(CHUNK));
        }
        if let Some(chunk) = self.chunks.last_mut() {
            chunk.push(value);
        }
        self.len += 1;
        self.len - 1
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.chunks.iter().flat_map(|c| c.iter())
    }

    /// 마지막 블록의 남는 용량을 반환한다.
    pub fn shrink_to_fit(&mut self) {
        if let Some(last) = self.chunks.last_mut() {
            last.shrink_to_fit();
        }
        self.chunks.shrink_to_fit();
    }
}

impl<T> Index<usize> for ChunkedVec<T> {
    type Output = T;

    #[inline]
    fn index(&self, i: usize) -> &T {
        &self.chunks[i >> CHUNK_BITS][i & (CHUNK - 1)]
    }
}

impl<T> IndexMut<usize> for ChunkedVec<T> {
    #[inline]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.chunks[i >> CHUNK_BITS][i & (CHUNK - 1)]
    }
}

const POOL_BITS: u32 = 20;
const POOL_CHUNK: usize = 1 << POOL_BITS;

/// 이름 문자열 풀. 1MB 블록에 이어 붙이며, 이름 하나가 블록 경계를 넘지 않는다.
/// 오프셋은 (블록 번호 << 20 | 블록 내 위치)로 인코딩된다 (최대 4GB).
#[derive(Clone)]
pub struct NamePool {
    chunks: Vec<Vec<u8>>,
}

impl NamePool {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// bytes 길이는 u16 범위여야 한다.
    pub fn push(&mut self, bytes: &[u8]) -> u32 {
        debug_assert!(bytes.len() <= u16::MAX as usize);
        if self.chunks.last().is_none_or(|c| c.len() + bytes.len() > POOL_CHUNK) {
            self.chunks.push(Vec::with_capacity(POOL_CHUNK));
        }
        let idx = self.chunks.len() - 1;
        let chunk = &mut self.chunks[idx];
        let off = chunk.len();
        chunk.extend_from_slice(bytes);
        ((idx as u32) << POOL_BITS) | off as u32
    }

    #[inline]
    pub fn get(&self, off: u32, len: u16) -> &[u8] {
        let chunk = &self.chunks[(off >> POOL_BITS) as usize];
        let start = (off as usize) & (POOL_CHUNK - 1);
        &chunk[start..start + len as usize]
    }

    pub fn shrink_to_fit(&mut self) {
        if let Some(last) = self.chunks.last_mut() {
            last.shrink_to_fit();
        }
        self.chunks.shrink_to_fit();
    }

    pub fn byte_len(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_vec_indexes_across_chunks() {
        let mut v = ChunkedVec::new();
        for i in 0..(CHUNK * 2 + 10) {
            assert_eq!(v.push(i), i);
        }
        assert_eq!(v.len(), CHUNK * 2 + 10);
        assert_eq!(v[CHUNK - 1], CHUNK - 1);
        assert_eq!(v[CHUNK], CHUNK);
        assert_eq!(v[CHUNK * 2 + 9], CHUNK * 2 + 9);
        v[CHUNK] = 0;
        assert_eq!(v[CHUNK], 0);
        assert_eq!(v.iter().count(), v.len());
    }

    #[test]
    fn name_pool_never_splits_a_name() {
        let mut pool = NamePool::new();
        let name = vec![b'n'; 60_000];
        let offs: Vec<u32> = (0..40).map(|_| pool.push(&name)).collect();
        assert!(pool.chunks.len() > 1);
        for off in offs {
            assert_eq!(pool.get(off, name.len() as u16), &name[..]);
        }
        let small = pool.push(b"tail");
        assert_eq!(pool.get(small, 4), b"tail");
    }
}
