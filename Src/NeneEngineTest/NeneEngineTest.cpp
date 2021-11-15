// NeneEngineTest.cpp : Defines the entry point for the application.
//

#include "NeneEngineTest/framework.h"
#include "NeneEngineTest/NeneEngineTest.hpp"
#include "NeneEngine/engine.hpp"

#define MAX_LOADSTRING 100

// Global Variables:
WCHAR szTitle[MAX_LOADSTRING];                  // The title bar text
WCHAR szWindowClass[MAX_LOADSTRING];            // the main window class name

int APIENTRY wWinMain(_In_ HINSTANCE hInstance,
                     _In_opt_ HINSTANCE hPrevInstance,
                     _In_ LPWSTR    lpCmdLine,
                     _In_ int       nCmdShow)
{
    // Initialize global strings
    LoadStringW(hInstance, IDS_APP_TITLE, szTitle, MAX_LOADSTRING);
    LoadStringW(hInstance, IDC_NENEENGINETEST, szWindowClass, MAX_LOADSTRING);

    bool res = Nene::Engine::Create(hInstance, nCmdShow, szTitle, szWindowClass);
    if (!res) {
        return FALSE;
    }

    Nene::Engine* neneEngine = Nene::Engine::GetInstance();

    int msgWParam = neneEngine->Run();

    neneEngine->Delete();

    return msgWParam;
}
