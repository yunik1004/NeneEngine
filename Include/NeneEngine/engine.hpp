#pragma once

#include <d3d12.h>
#include <dxgi1_4.h>
#include <wrl.h>

#ifndef NENE_MAX_LOADSTRING
#define NENE_MAX_LOADSTRING 100
#endif // !NENE_MAX_LOADSTRING


namespace Nene {
	class Engine {
	public:
		Engine(HINSTANCE hInstance, int nCmdShow, WCHAR* szTitle, WCHAR* szWindowClass);

		bool Init(void);
		int Run(void);

	protected:
		bool InitMainWindow(void);
		void InitDirect3D(void);

		/* Windows */
		virtual ATOM RegisterWndClass(void); // Registers the window class
		virtual BOOL InitWndInstance(void); // Saves instance handle and creates main window
		static LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam); // Processes messages for the main window

		HINSTANCE hInstance = nullptr; // current instance
		int nCmdShow = 0;
		WCHAR szTitle[NENE_MAX_LOADSTRING]; // The title bar text
		WCHAR szWindowClass[NENE_MAX_LOADSTRING]; // The main window class name
		HWND mhMainWnd = nullptr; // main window handle

		/* DirectX 12 */
		void CreateD3D12Device(void); // Create Direct3D 12 device
		void Check4xMsaaQualitySupport(void); // Check 4X MSAA quality support
		void CreateCommandObjects(void); // Create command queue and list
		void CreateSwapChain(void);
		void CreateRtvAndDsvDescriptorHeap(void);
		void CreateRtv(void);
		void CreateDepthStencilBufferAndView(void);

		D3D12_CPU_DESCRIPTOR_HANDLE CurrentBackBufferVIew(void) const;
		D3D12_CPU_DESCRIPTOR_HANDLE DepthStencilView(void) const;

		Microsoft::WRL::ComPtr<IDXGIFactory4> mdxgiFactory;
		Microsoft::WRL::ComPtr<IDXGISwapChain> mSwapChain;
		Microsoft::WRL::ComPtr<ID3D12Device> md3dDevice;
		Microsoft::WRL::ComPtr<ID3D12Fence> mFence;

		Microsoft::WRL::ComPtr<ID3D12CommandQueue> mCommandQueue;
		Microsoft::WRL::ComPtr<ID3D12CommandAllocator> mDirectCmdListAlloc;
		Microsoft::WRL::ComPtr<ID3D12GraphicsCommandList> mCommandList;

		static const int SwapChainBufferCount = 2;
		int mCurrBackBuffer = 0;
		Microsoft::WRL::ComPtr<ID3D12Resource> mSwapChainBuffer[SwapChainBufferCount];
		Microsoft::WRL::ComPtr<ID3D12Resource> mDepthStencilBuffer;

		Microsoft::WRL::ComPtr<ID3D12DescriptorHeap> mRtvHeap;
		Microsoft::WRL::ComPtr<ID3D12DescriptorHeap> mDsvHeap;

		// Descriptor size
		UINT mRtvDescriptorSize = 0;
		UINT mDsvDescriptorSize = 0;
		UINT mCbvSrvUavDescriptorSize = 0;

		// Set true to use 4X MSAA (?.1.8).  The default is false.
		bool      m4xMsaaState = false; // 4X MSAA enabled
		UINT m4xMsaaQuality = 0; // quality level of 4X MSAA

		DXGI_FORMAT mBackBufferFormat = DXGI_FORMAT_R8G8B8A8_UNORM;
		DXGI_FORMAT mDepthStencilFormat = DXGI_FORMAT_D24_UNORM_S8_UINT;

		int mClientWidth = 800;
		int mClientHeight = 600;
	};
}
