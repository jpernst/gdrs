#include <os/memory.h>


extern "C" {
	void* godot_rs_alloc(size_t p_bytes) {
#ifdef DEBUG_MEMORY_ENABLED
		return Memory::alloc_static(p_bytes, "rust");
#else
		return Memory::alloc_static(p_bytes, "");
#endif //DEBUG_MEMORY_ENABLED
	}


	void* godot_rs_realloc(void* p_ptr, size_t p_bytes)
	{
		return Memory::realloc_static(p_ptr, p_bytes);
	}


	void godot_rs_free(void* p_ptr) {
		return Memory::free_static(p_ptr);
	}
}
