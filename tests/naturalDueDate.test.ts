import { describe, expect, it } from "vitest";
import { extractNaturalDueDate } from "../src/domain/naturalDueDate";

const NOW = new Date(2026, 9, 14); // Wednesday, 14 October 2026

/** Friday 16 October 2026 — the weekday tests need their own «now». */
const FRIDAY = new Date(2026, 9, 16);

describe("the phrase from the letter the user wrote", () => {
  it("turns «проверить диски за неделю» into a phone-a-week deadline", () => {
    const { dueDate, cleanedTitle } = extractNaturalDueDate(
      "проверить диски за неделю",
      NOW,
    );
    expect(dueDate).toBe("2026-10-21");
    expect(cleanedTitle).toBe("проверить диски");
  });
});

describe("fixed names", () => {
  it("understands сегодня, завтра and послезавтра", () => {
    expect(extractNaturalDueDate("созвон сегодня", NOW).dueDate).toBe("2026-10-14");
    expect(extractNaturalDueDate("посадить цветы завтра", NOW).dueDate).toBe("2026-10-15");
    expect(extractNaturalDueDate("послезавтра созвон", NOW).dueDate).toBe("2026-10-16");
    expect(extractNaturalDueDate("послезавтра созвон", NOW).cleanedTitle).toBe("созвон");
  });

  it("works in english and keeps the english words out", () => {
    const r = extractNaturalDueDate("fix the driveway tomorrow", NOW);
    expect(r.dueDate).toBe("2026-10-15");
    expect(r.cleanedTitle).toBe("fix the driveway");
  });

  it("does not fire on words that merely contain a date name", () => {
    // «завтрак» / «тономorrow-case»: neither is a date.
    expect(extractNaturalDueDate("завтрак по пятницам", NOW)).toEqual({
      dueDate: null,
      cleanedTitle: "завтрак по пятницам",
    });
    expect(extractNaturalDueDate("tomorrowland tickets", NOW).dueDate).toBeNull();
  });
});

describe("durations", () => {
  it("counts days, weeks, months and years, digits or words", () => {
    expect(extractNaturalDueDate("сдать отчёт за 3 дня", NOW).dueDate).toBe("2026-10-17");
    expect(extractNaturalDueDate("встреча через 2 недели", NOW).dueDate).toBe("2026-10-28");
    expect(extractNaturalDueDate("съездить за месяц", NOW).dueDate).toBe("2026-11-14");
    expect(extractNaturalDueDate("отчёт через год", NOW).dueDate).toBe("2027-10-14");
    expect(extractNaturalDueDate("потратить на спринт в течение двух недель", NOW).dueDate).toBe(
      "2026-10-28",
    );
    expect(extractNaturalDueDate("отрисовка за десять дней", NOW).dueDate).toBe("2026-10-24");
  });

  it("reads the english in-forms", () => {
    expect(extractNaturalDueDate("sync in 3 days", NOW).dueDate).toBe("2026-10-17");
    expect(extractNaturalDueDate("sync in a week", NOW).dueDate).toBe("2026-10-21");
    expect(extractNaturalDueDate("plan in two weeks", NOW).dueDate).toBe("2026-10-28");
  });

  it("stays quiet away from за/через/в течение", () => {
    // «неделю», «пятницу» alone are words in a sentence, not deadlines.
    const r = extractNaturalDueDate("неделю назад закончил", NOW);
    expect(r.dueDate).toBeNull();
    expect(r.cleanedTitle).toBe("неделю назад закончил");
  });
});

describe("weekdays", () => {
  it("picks the nearest weekday, today counts when it already is that day", () => {
    // Wednesday → upcoming Friday is two days out.
    expect(extractNaturalDueDate("встреча в пт", NOW).dueDate).toBe("2026-10-16");
    expect(extractNaturalDueDate("встреча в вятницу", NOW).dueDate).toBeNull();
    expect(extractNaturalDueDate("созвон в пятницу", NOW).dueDate).toBe("2026-10-16");
    // Friday «now»: the day named is today.
    expect(extractNaturalDueDate("созвон в пятницу", FRIDAY).dueDate).toBe("2026-10-16");
    // «следующую» moves past today on purpose.
    expect(extractNaturalDueDate("созвон в следующую пятницу", FRIDAY).dueDate).toBe("2026-10-23");
  });

  it("maps the english weekdays the same way", () => {
    expect(extractNaturalDueDate("meet on friday", NOW).dueDate).toBe("2026-10-16");
    expect(extractNaturalDueDate("meet on fri", NOW).dueDate).toBe("2026-10-16");
    expect(extractNaturalDueDate("demo next monday", NOW).dueDate).toBe("2026-10-19");
    expect(extractNaturalDueDate("demo next monday", FRIDAY).dueDate).toBe("2026-10-19");
  });
});

describe("calendar dates", () => {
  it("reads dates without a year toward the future", () => {
    expect(extractNaturalDueDate("сдать 31.10", NOW).dueDate).toBe("2026-10-31");
    // March has passed: the nearest occurrence is next year.
    expect(extractNaturalDueDate("сдать 5.3", NOW).dueDate).toBe("2027-03-05");
  });

  it("keeps explicit years as typed and refuses impossible dates", () => {
    expect(extractNaturalDueDate("сдать 31.10.2027", NOW).dueDate).toBe("2027-10-31");
    const impossible = extractNaturalDueDate("сдать 31.02", NOW);
    expect(impossible.dueDate).toBeNull();
    expect(impossible.cleanedTitle).toBe("сдать 31.02");
    expect(extractNaturalDueDate("сдать до 2026-12-01", NOW).dueDate).toBe("2026-12-01");
  });

  it("does not swallow version numbers", () => {
    const r = extractNaturalDueDate("обновиться до 2.5.6", NOW);
    // «2.5» looks like a day and month; the version remainder («.6») must
    // keep the title intact and no date implied.
    expect(r.dueDate).toBeNull();
    expect(r.cleanedTitle).toBe("обновиться до 2.5.6");
  });
});

describe("the guard rails", () => {
  it("case does not matter, at any position of the phrase", () => {
    const r = extractNaturalDueDate("ЗАВТРА проверить диски", NOW);
    expect(r.dueDate).toBe("2026-10-15");
    expect(r.cleanedTitle).toBe("проверить диски");
  });

  it("a phrase that was the whole title leaves no task behind", () => {
    const r = extractNaturalDueDate("завтра", NOW);
    expect(r.dueDate).toBeNull();
    expect(r.cleanedTitle).toBe("завтра");
  });

  it("cuts the leftmost phrase and keeps the rest readable", () => {
    const r = extractNaturalDueDate("в пт созвон, за неделю до релиза", NOW);
    // «в пт» is earlier in the line, so it wins; the rest stays.
    expect(r.dueDate).toBe("2026-10-16");
    expect(r.cleanedTitle).toBe("созвон, за неделю до релиза");
  });
});
