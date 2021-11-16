// NeneEngineTest.cpp : Defines the entry point for the application.
//

#define NENE_MAX_LOADSTRING 100
#include "NeneEngineTest/NeneEngineTest.hpp"


int APIENTRY wWinMain(_In_ HINSTANCE hInstance,
                     _In_opt_ HINSTANCE hPrevInstance,
                     _In_ LPWSTR    lpCmdLine,
                     _In_ int       nCmdShow)
{
    WCHAR szTitle[NENE_MAX_LOADSTRING];                  // The title bar text
    WCHAR szWindowClass[NENE_MAX_LOADSTRING];            // the main window class name

    // Initialize global strings
    LoadStringW(hInstance, IDS_APP_TITLE, szTitle, NENE_MAX_LOADSTRING);
    LoadStringW(hInstance, IDC_NENEENGINETEST, szWindowClass, NENE_MAX_LOADSTRING);

    TestEngine* engine = new TestEngine(hInstance, nCmdShow, szTitle, szWindowClass);

    engine->Init();
    int msgWParam = engine->Run();

    delete engine;

    return msgWParam;
}
