#pragma once

#include "NeneEngine/engine.hpp"
#include "NeneEngineTest/resource.h"

class TestEngine : public Nene::Engine {
public:
    TestEngine(HINSTANCE hInstance, int nCmdShow, WCHAR* szTitle, WCHAR* szWindowClass)
        : Nene::Engine(hInstance, nCmdShow, szTitle, szWindowClass) {}

protected:
	virtual ATOM RegisterWndClass(void) override {
        WNDCLASSEXW wcex;

        wcex.cbSize = sizeof(WNDCLASSEX);

        wcex.style = CS_HREDRAW | CS_VREDRAW;
        wcex.lpfnWndProc = WndProc;
        wcex.cbClsExtra = 0;
        wcex.cbWndExtra = 0;
        wcex.hInstance = hInstance;
        wcex.hIcon = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_NENEENGINETEST));
        wcex.hCursor = LoadCursor(nullptr, IDC_ARROW);
        wcex.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
        wcex.lpszMenuName = MAKEINTRESOURCEW(IDC_NENEENGINETEST);
        wcex.lpszClassName = szWindowClass;
        wcex.hIconSm = LoadIcon(wcex.hInstance, MAKEINTRESOURCE(IDI_SMALL));

        return RegisterClassExW(&wcex);
	}
};
