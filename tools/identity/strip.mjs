// What of a response is compared: see `strip`.

/**
 * The response without the fields that identify the build rather than the answer, and without
 * the fields that the reference has not: the code of a failure (`error.code`) and of a warning
 * (`warnings[].code`, `contextWarnings[].code`), also in the answers of the entries of a request
 * (`results[]`). A code names what the kind and the message of the same failure or warning say,
 * which are compared; the tests of the module check the codes themselves.
 */
export function strip(response, request) {
  const r = withoutCodes({ ...response });
  delete r.elapsedMicros;
  delete r.engineCommit;
  if (request && request.op === 'info') delete r.version;
  return r;
}

/** `response` (an object, changed in place) without the codes of its failure and its warnings, and so its `results`. */
function withoutCodes(response) {
  const uncoded = (value) => {
    if (value === null || typeof value !== 'object' || Array.isArray(value) || !('code' in value)) return value;
    const { code: _, ...rest } = value;
    return rest;
  };
  if ('error' in response) response.error = uncoded(response.error);
  for (const list of ['warnings', 'contextWarnings']) if (Array.isArray(response[list])) response[list] = response[list].map(uncoded);
  if (Array.isArray(response.results)) response.results = response.results.map((entry) => (entry !== null && typeof entry === 'object' && !Array.isArray(entry) ? withoutCodes({ ...entry }) : entry));
  return response;
}
