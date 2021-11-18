#pragma once

#include <d3d12.h>
#include <exception>
#include <string>
#include <winerror.h>

namespace Nene {
    template <class T>
    constexpr auto& keep(T&& x) noexcept {
        return x;
    }

    // Helper class for COM exceptions
    class com_exception : public std::exception
    {
    public:
        com_exception(HRESULT hr) : result(hr) {}

        const char* what() const override {
            static char s_str[64] = {};
            sprintf_s(s_str, "Failure with HRESULT of %08X", static_cast<unsigned int>(result));
            return s_str;
        }

    private:
        HRESULT result;
    };

    // Helper utility converts D3D API failures into exceptions.
    inline void ThrowIfFailed(HRESULT hr) {
        if (FAILED(hr)) {
            throw com_exception(hr);
        }
    }

    /* d3dx12.h */
    struct CD3DX12_DEFAULT {};

    struct CD3DX12_CPU_DESCRIPTOR_HANDLE : public D3D12_CPU_DESCRIPTOR_HANDLE
    {
        CD3DX12_CPU_DESCRIPTOR_HANDLE() = default;
        explicit CD3DX12_CPU_DESCRIPTOR_HANDLE(const D3D12_CPU_DESCRIPTOR_HANDLE& o) noexcept :
            D3D12_CPU_DESCRIPTOR_HANDLE(o)
        {}
        CD3DX12_CPU_DESCRIPTOR_HANDLE(CD3DX12_DEFAULT) noexcept { ptr = 0; }
        CD3DX12_CPU_DESCRIPTOR_HANDLE(_In_ const D3D12_CPU_DESCRIPTOR_HANDLE& other, INT offsetScaledByIncrementSize) noexcept
        {
            InitOffsetted(other, offsetScaledByIncrementSize);
        }
        CD3DX12_CPU_DESCRIPTOR_HANDLE(_In_ const D3D12_CPU_DESCRIPTOR_HANDLE& other, INT offsetInDescriptors, UINT descriptorIncrementSize) noexcept
        {
            InitOffsetted(other, offsetInDescriptors, descriptorIncrementSize);
        }
        CD3DX12_CPU_DESCRIPTOR_HANDLE& Offset(INT offsetInDescriptors, UINT descriptorIncrementSize) noexcept
        {
            ptr = SIZE_T(INT64(ptr) + INT64(offsetInDescriptors) * INT64(descriptorIncrementSize));
            return *this;
        }
        CD3DX12_CPU_DESCRIPTOR_HANDLE& Offset(INT offsetScaledByIncrementSize) noexcept
        {
            ptr = SIZE_T(INT64(ptr) + INT64(offsetScaledByIncrementSize));
            return *this;
        }
        bool operator==(_In_ const D3D12_CPU_DESCRIPTOR_HANDLE& other) const noexcept
        {
            return (ptr == other.ptr);
        }
        bool operator!=(_In_ const D3D12_CPU_DESCRIPTOR_HANDLE& other) const noexcept
        {
            return (ptr != other.ptr);
        }
        CD3DX12_CPU_DESCRIPTOR_HANDLE& operator=(const D3D12_CPU_DESCRIPTOR_HANDLE& other) noexcept
        {
            ptr = other.ptr;
            return *this;
        }

        inline void InitOffsetted(_In_ const D3D12_CPU_DESCRIPTOR_HANDLE& base, INT offsetScaledByIncrementSize) noexcept
        {
            InitOffsetted(*this, base, offsetScaledByIncrementSize);
        }

        inline void InitOffsetted(_In_ const D3D12_CPU_DESCRIPTOR_HANDLE& base, INT offsetInDescriptors, UINT descriptorIncrementSize) noexcept
        {
            InitOffsetted(*this, base, offsetInDescriptors, descriptorIncrementSize);
        }

        static inline void InitOffsetted(_Out_ D3D12_CPU_DESCRIPTOR_HANDLE& handle, _In_ const D3D12_CPU_DESCRIPTOR_HANDLE& base, INT offsetScaledByIncrementSize) noexcept
        {
            handle.ptr = SIZE_T(INT64(base.ptr) + INT64(offsetScaledByIncrementSize));
        }

        static inline void InitOffsetted(_Out_ D3D12_CPU_DESCRIPTOR_HANDLE& handle, _In_ const D3D12_CPU_DESCRIPTOR_HANDLE& base, INT offsetInDescriptors, UINT descriptorIncrementSize) noexcept
        {
            handle.ptr = SIZE_T(INT64(base.ptr) + INT64(offsetInDescriptors) * INT64(descriptorIncrementSize));
        }
    };

    struct CD3DX12_RESOURCE_BARRIER : public D3D12_RESOURCE_BARRIER
    {
        CD3DX12_RESOURCE_BARRIER() = default;
        explicit CD3DX12_RESOURCE_BARRIER(const D3D12_RESOURCE_BARRIER& o) noexcept :
            D3D12_RESOURCE_BARRIER(o)
        {}
        static inline CD3DX12_RESOURCE_BARRIER Transition(
            _In_ ID3D12Resource* pResource,
            D3D12_RESOURCE_STATES stateBefore,
            D3D12_RESOURCE_STATES stateAfter,
            UINT subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            D3D12_RESOURCE_BARRIER_FLAGS flags = D3D12_RESOURCE_BARRIER_FLAG_NONE) noexcept
        {
            CD3DX12_RESOURCE_BARRIER result = {};
            D3D12_RESOURCE_BARRIER& barrier = result;
            result.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
            result.Flags = flags;
            barrier.Transition.pResource = pResource;
            barrier.Transition.StateBefore = stateBefore;
            barrier.Transition.StateAfter = stateAfter;
            barrier.Transition.Subresource = subresource;
            return result;
        }
        static inline CD3DX12_RESOURCE_BARRIER Aliasing(
            _In_ ID3D12Resource* pResourceBefore,
            _In_ ID3D12Resource* pResourceAfter) noexcept
        {
            CD3DX12_RESOURCE_BARRIER result = {};
            D3D12_RESOURCE_BARRIER& barrier = result;
            result.Type = D3D12_RESOURCE_BARRIER_TYPE_ALIASING;
            barrier.Aliasing.pResourceBefore = pResourceBefore;
            barrier.Aliasing.pResourceAfter = pResourceAfter;
            return result;
        }
        static inline CD3DX12_RESOURCE_BARRIER UAV(
            _In_ ID3D12Resource* pResource) noexcept
        {
            CD3DX12_RESOURCE_BARRIER result = {};
            D3D12_RESOURCE_BARRIER& barrier = result;
            result.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV;
            barrier.UAV.pResource = pResource;
            return result;
        }
    };

    struct CD3DX12_HEAP_PROPERTIES : public D3D12_HEAP_PROPERTIES
    {
        CD3DX12_HEAP_PROPERTIES() = default;
        explicit CD3DX12_HEAP_PROPERTIES(const D3D12_HEAP_PROPERTIES& o) noexcept :
            D3D12_HEAP_PROPERTIES(o)
        {}
        CD3DX12_HEAP_PROPERTIES(
            D3D12_CPU_PAGE_PROPERTY cpuPageProperty,
            D3D12_MEMORY_POOL memoryPoolPreference,
            UINT creationNodeMask = 1,
            UINT nodeMask = 1) noexcept
        {
            Type = D3D12_HEAP_TYPE_CUSTOM;
            CPUPageProperty = cpuPageProperty;
            MemoryPoolPreference = memoryPoolPreference;
            CreationNodeMask = creationNodeMask;
            VisibleNodeMask = nodeMask;
        }
        explicit CD3DX12_HEAP_PROPERTIES(
            D3D12_HEAP_TYPE type,
            UINT creationNodeMask = 1,
            UINT nodeMask = 1) noexcept
        {
            Type = type;
            CPUPageProperty = D3D12_CPU_PAGE_PROPERTY_UNKNOWN;
            MemoryPoolPreference = D3D12_MEMORY_POOL_UNKNOWN;
            CreationNodeMask = creationNodeMask;
            VisibleNodeMask = nodeMask;
        }
        bool IsCPUAccessible() const noexcept
        {
            return Type == D3D12_HEAP_TYPE_UPLOAD || Type == D3D12_HEAP_TYPE_READBACK || (Type == D3D12_HEAP_TYPE_CUSTOM &&
                (CPUPageProperty == D3D12_CPU_PAGE_PROPERTY_WRITE_COMBINE || CPUPageProperty == D3D12_CPU_PAGE_PROPERTY_WRITE_BACK));
        }
    };
    inline bool operator==(const D3D12_HEAP_PROPERTIES& l, const D3D12_HEAP_PROPERTIES& r) noexcept
    {
        return l.Type == r.Type && l.CPUPageProperty == r.CPUPageProperty &&
            l.MemoryPoolPreference == r.MemoryPoolPreference &&
            l.CreationNodeMask == r.CreationNodeMask &&
            l.VisibleNodeMask == r.VisibleNodeMask;
    }
    inline bool operator!=(const D3D12_HEAP_PROPERTIES& l, const D3D12_HEAP_PROPERTIES& r) noexcept
    {
        return !(l == r);
    }
}
