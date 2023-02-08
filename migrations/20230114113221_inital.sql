CREATE TABLE player (
	id INTEGER PRIMARY KEY
);

CREATE TABLE score_sheet (
    id INTEGER PRIMARY KEY,
	msg_id INTEGER NOT NULL,
	day INTEGER NOT NULL,
	player_id INTEGER NOT NULL,
	score INTEGER NOT NULL,
	FOREIGN KEY(player_id) REFERENCES player(id),
	FOREIGN KEY(day) REFERENCES daily(id),
	UNIQUE(player_id, day)
);

CREATE TABLE daily (
	id INTEGER PRIMARY KEY,
	gold INTEGER,
	silver INTEGER,
	bronze INTEGER
);
