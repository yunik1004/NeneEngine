#include "NeneEngine/engine.hpp"
#include <cassert>
#include <d3d12sdklayers.h>
#include <dxgi.h>
#include "NeneEngine/util.hpp"


using namespace Microsoft::WRL;

namespace Nene {
	Engine::Engine(void) {
		InitDirect3D();
	}

	Engine::~Engine(void) {
		//
	}

	void Engine::InitMainWindow(void) {

	}

	void Engine::InitDirect3D(void) {
		CreateD3D12Device();

		/* Create fence for the synchronization of CPU and GPU */
		ThrowIfFailed(md3dDevice->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&mFence)));

		/* Get the size of descriptors */
		mRtvDescriptorSize = md3dDevice->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
		mDsvDescriptorSize = md3dDevice->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_DSV);
		mCbvSrvUavDescriptorSize = md3dDevice->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);

		Check4xMsaaQualitySupport();

		CreateCommandObjects();
		CreateSwapChain();
	}

	void Engine::CreateD3D12Device(void) {
#if defined(DEBUG) || defined(_DEBUG)
		{
			ComPtr<ID3D12Debug> debugController;
			ThrowIfFailed(D3D12GetDebugInterface(IID_PPV_ARGS(&debugController)));
			debugController->EnableDebugLayer();
		}
#endif

		ThrowIfFailed(CreateDXGIFactory1(IID_PPV_ARGS(&mdxgiFactory)));

		// Try to create hardware device
		HRESULT hardwareResult = D3D12CreateDevice(
			nullptr, // default
			D3D_FEATURE_LEVEL_11_0,
			IID_PPV_ARGS(&md3dDevice)
		);

		// Fallback to WARP device
		if (FAILED(hardwareResult)) {
			ComPtr<IDXGIAdapter> pWarpAdapter;
			ThrowIfFailed(mdxgiFactory->EnumWarpAdapter(IID_PPV_ARGS(&pWarpAdapter)));

			ThrowIfFailed(D3D12CreateDevice(
				pWarpAdapter.Get(),
				D3D_FEATURE_LEVEL_11_0,
				IID_PPV_ARGS(&md3dDevice)
			));
		}
	}

	void Engine::Check4xMsaaQualitySupport(void) {
		D3D12_FEATURE_DATA_MULTISAMPLE_QUALITY_LEVELS msQualityLevels;
		msQualityLevels.Format = mBackBufferFormat;
		msQualityLevels.SampleCount = 4;
		msQualityLevels.Flags = D3D12_MULTISAMPLE_QUALITY_LEVELS_FLAG_NONE;
		msQualityLevels.NumQualityLevels = 0;
		ThrowIfFailed(md3dDevice->CheckFeatureSupport(
			D3D12_FEATURE_MULTISAMPLE_QUALITY_LEVELS,
			&msQualityLevels,
			sizeof(msQualityLevels)
		));

		m4xMsaaQuality = msQualityLevels.NumQualityLevels;
		assert(m4xMsaaQuality > 0 && "Unexpected MSAA quality level");
	}

	void Engine::CreateCommandObjects(void) {
		D3D12_COMMAND_QUEUE_DESC queueDesc = {};
		queueDesc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
		queueDesc.Flags = D3D12_COMMAND_QUEUE_FLAG_NONE;
		ThrowIfFailed(md3dDevice->CreateCommandQueue(&queueDesc, IID_PPV_ARGS(&mCommandQueue)));

		ThrowIfFailed(md3dDevice->CreateCommandAllocator(
			D3D12_COMMAND_LIST_TYPE_DIRECT,
			IID_PPV_ARGS(mDirectCmdListAlloc.GetAddressOf())
		));

		ThrowIfFailed(md3dDevice->CreateCommandList(
			0,
			D3D12_COMMAND_LIST_TYPE_DIRECT,
			mDirectCmdListAlloc.Get(), // Associated command allocator
			nullptr, // Initial PipelineStateObject
			IID_PPV_ARGS(mCommandList.GetAddressOf())
		));

		// Start off in a closed state.  This is because the first time we refer 
		// to the command list we will Reset it, and it needs to be closed before
		// calling Reset.
		mCommandList->Close();
	}

	void Engine::CreateSwapChain(void) {
		// Release the previous swapchain
		mSwapChain.Reset();

		/*
		DXGI_SWAP_CHAIN_DESC sd;
		sd.BufferDesc.Width = mClientWidth;
		sd.BufferDesc.Height = mClientHeight;
		sd.BufferDesc.RefreshRate.Numerator = 60;
		sd.BufferDesc.RefreshRate.Denominator = 1;
		sd.BufferDesc.Format = mBackBufferFormat;
		sd.BufferDesc.ScanlineOrdering = DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED;
		sd.BufferDesc.Scaling = DXGI_MODE_SCALING_UNSPECIFIED;
		sd.SampleDesc.Count = m4xMsaaState ? 4 : 1;
		sd.SampleDesc.Quality = m4xMsaaState ? (m4xMsaaQuality - 1) : 0;
		sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
		sd.BufferCount = SwapChainBufferCount;
		sd.OutputWindow = mhMainWnd;
		sd.Windowed = true;
		sd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
		sd.Flags = DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH;

		ThrowIfFailed(mdxgiFactory->CreateSwapChain(
			mCommandQueue.Get(),
			&sd,
			mSwapChain.GetAddressOf()
		));
		*/
	}
}
