import { IJsonModel } from "@qstudio/flex-layout";

const DEFAULT_DOCK_MODEL: IJsonModel = {
  global: { "tabEnableFloat": false, enableRotateBorderIcons: false, tabSetEnableMaximize: false },
  "borders": [
      {
          "type": "border",
          "location": "bottom",
          "selected": 0,
          "size": 180,
          "children": [
            {
                "type": "tab",
                "enableClose": false,
                "enableRename": false,
                "name": "Log",
                "component": "Log",
                "icon": "images/bar_chart.svg"
            },
          ]
      },
      {
          "type": "border",
          "location": "left",
          "minSize": 200,
          "size": 300,
          "selected": 0,
          "barSize": 50,
          "children": [

              {
                  "type": "tab",
                  "enableClose": false,
                  "enableRename": false,
                  "name": "",
                  "altName": "",
                  "component": "FileExplorer",
                  "icon": "images/folder.svg"
              },
          ]
      },

  ],
  layout: {
      type: "row",
      weight: 100,
      children: [
          {
              type: "tabset",
              weight: 100,
              children: [
                {
                    type: "tab",
                    name: "Welcome",
                    enableRename: false,
                    component: "Welcome",
                },
                {
                    type: "tab",
                    name: "Block Planner",
                    enableRename: false,
                    component: "BlockPlanner",
                },
                {
                    type: "tab",
                    name: "Wallet",
                    enableRename: false,
                    component: "Wallet",
                },
                  {
                      "type": "tab",
                      "enableClose": true,
                      "enableRename": false,
                      "name": "Stage",
                      "component": "Stage",
                  },
              ]
          }
      ]
  }
};


export {
  DEFAULT_DOCK_MODEL,
}