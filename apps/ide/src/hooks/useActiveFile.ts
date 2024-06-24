import { create } from 'zustand'

interface IActiveFile {
  activeFile: string;
  setActiveFile: (activeFile: string) => any;
  
}
const useActiveFile = create<IActiveFile>(set => ({
  activeFile: "",
  setActiveFile: (activeFile: string) => set((state) =>{
    


    
    return { activeFile };
  }),
}))
export {
  useActiveFile
}