ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@fastmail.com header.s=fm2 header.b=9iLihtf9;
       dkim=pass header.i=@messagingengine.com header.s=fm2 header.b=vFQyciDG;
       spf=pass (google.com: domain of alice@fastmail.com designates 66.111.4.28 as permitted sender) smtp.mailfrom=alice@fastmail.com;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=fastmail.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@fastmail.com header.s=fm2 header.b=9iLihtf9;
       dkim=pass header.i=@messagingengine.com header.s=fm2 header.b=vFQyciDG;
       spf=pass (google.com: domain of alice@fastmail.com designates 66.111.4.28 as permitted sender) smtp.mailfrom=alice@fastmail.com;
       dmarc=pass (p=NONE sp=NONE dis=NONE) header.from=fastmail.com
From: <alice@fastmail.com>
To: <alice@gmail.com>
