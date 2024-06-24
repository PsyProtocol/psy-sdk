import {uuidv4} from '@qstudio/utils';

function getFileExtForFileName(fileName: string){
  const dotInd = fileName.lastIndexOf(".");
  if(dotInd===-1){
    return "";
  }else{
    return fileName.substring(dotInd+1);
  }
}
function getFileNameForFilePath(filePath: string){
  const slashInd = filePath.lastIndexOf("/");
  return filePath.substring(slashInd+1);
}
function getFileExtForFilePath(filePath: string){
  const slashInd = filePath.lastIndexOf("/");
  return getFileExtForFileName(filePath.substring(slashInd+1));
}

export {
  getFileExtForFileName,
  getFileExtForFilePath,
  getFileNameForFilePath,
}