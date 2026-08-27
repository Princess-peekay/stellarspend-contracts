import {SearchService} from './search.service';
it('empty',()=>expect(new SearchService().search('').toBe(''));
it('special',()=>expect(new SearchService().search('50%_off').toBe('50\%\__off'));
