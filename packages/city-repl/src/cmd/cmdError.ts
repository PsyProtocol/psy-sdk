class REPLParseError extends Error {
  fieldName: string;
  value: any;
  constructor(fieldName: string, value: any, message: string) {
    super(`Error parsing field '${fieldName}', invalid value ${value}: ${message}`); // (1)
    this.name = "REPLParseError";
    this.fieldName = fieldName;
    this.value = value;
  }
}

export {
  REPLParseError,
}